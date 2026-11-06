//! Settings (⌘,): how Inkling reads and looks, your minds, what it knows about
//! you, and which model it uses.

use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::*;
use gpui_kit::prelude::*;

use crate::brain::Brain;
use crate::store::{
    Appearance, Frequency, InkColor, Lettering, Mind, Mode, Pace, Placement, Store, TextSize, Voice, mind_templates,
    new_id,
};
use crate::theme::{SANS, SERIF, Theme, ink_color, note_size};
use crate::ui::{self, option_card, pill, primary_button, section_label};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Customize,
    Minds,
    NewMind,
    AboutYou,
    Model,
    Help,
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
                .auto_grow(4, 10)
                .placeholder("Nothing yet. Write what Inkling should know: what you do, what you're working toward, how you like feedback.");
            t.set_value(s.about_me.clone(), window, cx);
            t
        });
        let model = cx.new(|cx| {
            let mut i = InputState::new(window, cx).placeholder("claude-opus-5-5");
            i.set_value(s.model.clone(), window, cx);
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
        // The save button follows whether the name is filled in.
        subs.push(cx.subscribe(&name, |_, _, ev: &InputEvent, cx| {
            if let InputEvent::Change = ev {
                cx.notify();
            }
        }));
        subs.push(cx.subscribe_in(&model, window, |_, model, ev: &InputEvent, _, cx| {
            if let InputEvent::Change = ev {
                let v = model.read(cx).value().trim().to_string();
                if !v.is_empty() {
                    let store = cx.global_mut::<Store>();
                    store.settings.model = v;
                    store.save_settings();
                }
            }
        }));
        let mut this = Self {
            focus: cx.focus_handle(),
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
            _subs: subs,
        };
        if tab == Tab::NewMind {
            this.start_mind(None, window, cx);
        } else {
            this.tab = tab;
        }
        this
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

    fn customize(&mut self, cx: &mut Context<Self>) -> Div {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let s = cx.global::<Store>().settings.clone();
        let row = || div().w_full().mt(px(8.)).flex().flex_row().gap(px(8.));
        let section = |title: &'static str| div().mt(px(20.)).child(section_label(title, &t));

        let mut colors = div().mt(px(10.)).flex().gap(px(10.));
        for (i, ink) in InkColor::ALL.iter().enumerate() {
            let c = ink_color(*ink, &p);
            let on = s.ink == *ink;
            let ink = *ink;
            colors = colors.child(
                div()
                    .id(("ink", i))
                    .size(px(22.))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_1()
                    .border_color(if on { p.ink } else { p.paper })
                    .cursor_pointer()
                    .child(div().size(px(14.)).rounded_full().bg(c))
                    .on_click(cx.listener(move |this, _, _, cx| this.update(cx, |s| s.ink = ink))),
            );
        }

        div()
            .child(div().text_size(px(15.)).text_color(p.ink).child("Customize Inkling"))
            .child(section("New notes start in"))
            .child(
                row()
                    .child(option_card("m-think", "Reads along", "Reads along and speaks up when something's worth a second look.", s.default_mode == Mode::ThinksWithYou, &t)
                        .on_click(cx.listener(|this, _, _, cx| this.update(cx, |s| s.default_mode = Mode::ThinksWithYou))))
                    .child(option_card("m-ask", "Waits to be asked", "Quiet until you say its name, select words, or open the chat.", s.default_mode == Mode::WhenYouAsk, &t)
                        .on_click(cx.listener(|this, _, _, cx| this.update(cx, |s| s.default_mode = Mode::WhenYouAsk)))),
            )
            .child(div().mt(px(5.)).text_size(px(10.5)).text_color(p.faint).child("Change any note on its own, from the row above its title."))
            .child(
                div()
                    .mt(px(18.))
                    .p(px(10.))
                    .rounded(px(9.))
                    .border_1()
                    .border_color(p.rule)
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .child(div().text_size(px(12.5)).text_color(p.ink).child("Ink from the pointer"))
                            .child(div().text_size(px(11.)).text_color(p.graphite).child("Now and then a drop of ink falls as you move the pointer, then dries and fades.")),
                    )
                    .child(div().w(px(12.)))
                    .child(switch("ink-pointer", s.ink_from_pointer, &t).on_click(cx.listener(|this, _, _, cx| this.update(cx, |s| s.ink_from_pointer = !s.ink_from_pointer)))),
            )
            .child(section("Reading pace"))
            .child(
                row()
                    .child(option_card("pace-r", "Regular", "Reads a moment after you stop typing.", s.pace == Pace::Regular, &t)
                        .on_click(cx.listener(|this, _, _, cx| this.update(cx, |s| s.pace = Pace::Regular))))
                    .child(option_card("pace-s", "Slow", "Waits about twice as long, for when you pause mid-thought.", s.pace == Pace::Slow, &t)
                        .on_click(cx.listener(|this, _, _, cx| this.update(cx, |s| s.pace = Pace::Slow)))),
            )
            .child(section("Where questions appear"))
            .child(
                row()
                    .child(option_card("pl-b", "Both margins", "Your words in the middle, questions branch to either side.", s.placement == Placement::Both, &t)
                        .on_click(cx.listener(|this, _, _, cx| this.update(cx, |s| s.placement = Placement::Both))))
                    .child(option_card("pl-r", "Right margin", "Every question in one column beside your writing.", s.placement == Placement::Right, &t)
                        .on_click(cx.listener(|this, _, _, cx| this.update(cx, |s| s.placement = Placement::Right))))
                    .child(option_card("pl-m", "Marks only", "A small ? after the passage. Open it when you're ready.", s.placement == Placement::Marks, &t)
                        .on_click(cx.listener(|this, _, _, cx| this.update(cx, |s| s.placement = Placement::Marks)))),
            )
            .child(section("Appearance"))
            .child(
                row()
                    .child(option_card("ap-s", "System", "Follows macOS.", s.appearance == Appearance::System, &t)
                        .on_click(cx.listener(|this, _, _, cx| this.update(cx, |s| s.appearance = Appearance::System))))
                    .child(option_card("ap-l", "Light", "Paper and ink.", s.appearance == Appearance::Light, &t)
                        .on_click(cx.listener(|this, _, _, cx| this.update(cx, |s| s.appearance = Appearance::Light))))
                    .child(option_card("ap-d", "Dark", "Ink and paper.", s.appearance == Appearance::Dark, &t)
                        .on_click(cx.listener(|this, _, _, cx| this.update(cx, |s| s.appearance = Appearance::Dark)))),
            )
            .child(section("Text size"))
            .child({
                let mut r = row();
                for (i, &size) in TextSize::ALL.iter().enumerate() {
                    let on = s.text_size == size;
                    r = r.child(
                        div()
                            .id(("size", i))
                            .flex_1()
                            .px(px(10.))
                            .py(px(9.))
                            .rounded(px(9.))
                            .border_1()
                            .border_color(if on { p.ink } else { p.rule })
                            .cursor_pointer()
                            .child(div().text_size(note_size(size)).text_color(p.ink).child("A thought."))
                            .child(div().text_size(px(10.5)).text_color(p.graphite).child(size.label()))
                            .on_click(cx.listener(move |this, _, _, cx| this.update(cx, |s| s.text_size = size))),
                    );
                }
                r
            })
            .child(section("Inkling's lettering"))
            .child(
                row()
                    .child(
                        div()
                            .id("let-own")
                            .flex_1()
                            .px(px(10.))
                            .py(px(9.))
                            .rounded(px(9.))
                            .border_1()
                            .border_color(if s.lettering == Lettering::Own { p.ink } else { p.rule })
                            .cursor_pointer()
                            .child(div().font_family(SANS).text_size(px(15.)).text_color(p.ink).child("Does this still hold?"))
                            .child(div().text_size(px(10.5)).text_color(p.graphite).child("Inkling's own"))
                            .on_click(cx.listener(|this, _, _, cx| this.update(cx, |s| s.lettering = Lettering::Own))),
                    )
                    .child(
                        div()
                            .id("let-notes")
                            .flex_1()
                            .px(px(10.))
                            .py(px(9.))
                            .rounded(px(9.))
                            .border_1()
                            .border_color(if s.lettering == Lettering::LikeNotes { p.ink } else { p.rule })
                            .cursor_pointer()
                            .child(div().font_family(SERIF).text_size(px(15.)).text_color(p.ink).child("Does this still hold?"))
                            .child(div().text_size(px(10.5)).text_color(p.graphite).child("Like your notes"))
                            .on_click(cx.listener(|this, _, _, cx| this.update(cx, |s| s.lettering = Lettering::LikeNotes))),
                    ),
            )
            .child(section("Inkling's colour"))
            .child(colors)
    }

    fn minds(&mut self, cx: &mut Context<Self>) -> Div {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let minds = cx.global::<Store>().minds.clone();
        let mut list = div().mt(px(14.)).flex().flex_col().gap(px(12.));
        for (i, m) in minds.iter().enumerate() {
            if i == 1 {
                list = list.child(div().mt(px(6.)).text_size(px(10.5)).text_color(p.faint).child("Yours"));
            }
            let mind = m.clone();
            list = list.child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(8.))
                    .child(div().pt(px(6.)).child(ui::small_dot(p.ink, 5.)))
                    .child(
                        div()
                            .flex_1()
                            .child(div().text_size(px(12.5)).text_color(p.ink).child(m.name.clone()))
                            .child(div().text_size(px(11.)).text_color(p.graphite).child(m.description.clone())),
                    )
                    .when(i > 0, |d| {
                        d.child(ui::ghost_button(("edit-mind", i), "Edit", &t).on_click(cx.listener(move |this, _, window, cx| {
                            this.start_mind(Some(mind.clone()), window, cx)
                        })))
                    }),
            );
        }
        if minds.len() == 1 {
            list = list
                .child(div().mt(px(6.)).text_size(px(10.5)).text_color(p.faint).child("Yours"))
                .child(div().text_size(px(11.5)).text_color(p.graphite).child("None yet. Start from a staff engineer, an editor, a tutor, or write your own."));
        }
        div()
            .child(
                div()
                    .flex()
                    .items_start()
                    .child(
                        div()
                            .flex_1()
                            .child(div().text_size(px(15.)).text_color(p.ink).child("Minds"))
                            .child(div().mt(px(4.)).text_size(px(11.5)).text_color(p.graphite).child("Minds that read your notes, each with its own way of asking. Pick one for any note from the row above its title.")),
                    )
                    .child(primary_button("new-mind", "New mind", &t).on_click(cx.listener(|this, _, window, cx| this.start_mind(None, window, cx)))),
            )
            .child(list)
    }

    fn mind_form(&mut self, cx: &mut Context<Self>) -> Div {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let templates = mind_templates();
        let mut chips = div().mt(px(6.)).flex().flex_wrap().gap(px(6.));
        for (i, tm) in templates.iter().enumerate() {
            let tm = tm.clone();
            chips = chips.child(pill(("tmpl", i), tm.name.clone(), self.template == Some(i), &t).on_click(cx.listener(
                move |this, _, window, cx| {
                    this.template = Some(i);
                    let m = tm.clone();
                    this.fill(&m, window, cx);
                },
            )));
        }
        let field = |label: &'static str, hint: &'static str| {
            div()
                .mt(px(14.))
                .flex()
                .items_baseline()
                .gap(px(6.))
                .child(div().text_size(px(12.)).text_color(p.ink).child(label))
                .child(div().text_size(px(10.5)).text_color(p.faint).child(hint))
        };
        let boxed = |child: AnyElement| {
            div()
                .mt(px(5.))
                .px(px(8.))
                .py(px(5.))
                .rounded(px(7.))
                .border_1()
                .border_color(p.rule)
                .text_size(px(12.5))
                .child(child)
        };
        let mut voices = div().mt(px(6.)).flex().gap(px(6.));
        for (i, v) in Voice::ALL.iter().enumerate() {
            let v = *v;
            voices = voices.child(pill(("voice", i), v.label(), self.voice == v, &t).on_click(cx.listener(move |this, _, _, cx| {
                this.voice = v;
                cx.notify();
            })));
        }
        let mut freqs = div().mt(px(6.)).flex().gap(px(6.));
        for (i, f) in Frequency::ALL.iter().enumerate() {
            let f = *f;
            freqs = freqs.child(pill(("freq", i), f.label(), self.frequency == f, &t).on_click(cx.listener(move |this, _, _, cx| {
                this.frequency = f;
                cx.notify();
            })));
        }
        let editing = self.editing.is_some();
        let named = !self.name.read(cx).value().trim().is_empty();
        div()
            .child(div().flex().items_center().gap(px(7.)).child(ui::small_dot(p.ink, 6.)).child(
                div().text_size(px(15.)).text_color(p.ink).child(if editing { "Edit mind" } else { "New mind" }),
            ))
            .child(div().mt(px(4.)).text_size(px(11.5)).text_color(p.graphite).child("A mind is what reads your notes: what it looks for, and how it asks. Give any note the one it needs."))
            .when(!editing, |d| {
                d.child(div().mt(px(14.)).text_size(px(10.5)).text_color(p.faint).child("Start from")).child(chips)
            })
            .child(field("Name", ""))
            .child(boxed(Input::new(&self.name).appearance(false).into_any_element()))
            .child(field("Description", "Shown when you pick it"))
            .child(boxed(Input::new(&self.description).appearance(false).into_any_element()))
            .child(field("How it thinks", "What it looks for, pushes on, and leaves alone"))
            .child(boxed(Textarea::new(&self.how).appearance(false).into_any_element()))
            .child(field("Voice", ""))
            .child(voices)
            .child(field("How often it speaks up", ""))
            .child(freqs)
            .child(
                div()
                    .mt(px(20.))
                    .flex()
                    .items_center()
                    .gap(px(14.))
                    .when(editing, |d| {
                        d.child(ui::ghost_button("delete-mind", "Delete", &t).on_click(cx.listener(|this, _, _, cx| this.delete_mind(cx))))
                    })
                    .child(div().flex_1())
                    .child(ui::ghost_button("cancel-mind", "Cancel", &t).on_click(cx.listener(|this, _, _, cx| {
                        this.tab = Tab::Minds;
                        this.editing = None;
                        cx.notify();
                    })))
                    .child(
                        primary_button("save-mind", if editing { "Save" } else { "Create mind" }, &t)
                            .when(!named, |b| b.opacity(0.4).cursor_default())
                            .on_click(cx.listener(|this, _, _, cx| this.save_mind(cx))),
                    ),
            )
    }

    fn about(&mut self, cx: &mut Context<Self>) -> Div {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let s = cx.global::<Store>().settings.clone();
        div()
            .child(div().text_size(px(15.)).text_color(p.ink).child("About you"))
            .child(div().mt(px(4.)).text_size(px(11.5)).text_color(p.graphite).child(
                "A short paragraph Inkling keeps about you, so every note starts out knowing it. It stays on this Mac.",
            ))
            .child(
                div()
                    .mt(px(14.))
                    .p(px(10.))
                    .rounded(px(9.))
                    .border_1()
                    .border_color(p.rule)
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .child(div().text_size(px(12.5)).text_color(p.ink).child("Inkling learns about you"))
                            .child(div().text_size(px(11.)).text_color(p.graphite).child(
                                "When you say something lasting about yourself, like what you study or how you like feedback, it can be worked in here.",
                            )),
                    )
                    .child(div().w(px(12.)))
                    .child(switch("learns", s.learns_about_me, &t).on_click(cx.listener(|this, _, _, cx| {
                        this.update(cx, |s| s.learns_about_me = !s.learns_about_me)
                    }))),
            )
            .child(
                div()
                    .mt(px(12.))
                    .px(px(10.))
                    .py(px(8.))
                    .rounded(px(9.))
                    .border_1()
                    .border_color(p.rule)
                    .text_size(px(12.5))
                    .child(Textarea::new(&self.about).appearance(false)),
            )
            .child(div().mt(px(5.)).text_size(px(10.5)).text_color(p.faint).child(format!("{} of {ABOUT_LIMIT}", s.about_me.chars().count())))
    }

    fn model_tab(&mut self, cx: &mut Context<Self>) -> Div {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let s = cx.global::<Store>().settings.clone();
        let brain = Brain::global(cx).read(cx);
        let account = brain.account.clone();
        let limit = brain.limit.clone();
        let state = brain.state.clone();
        let last_error = brain.last_error.clone();
        let status = match (&state, &account) {
            (crate::brain::BrainState::Failed(e), _) => format!("Not running: {e}"),
            (_, Some(a)) => format!(
                "Signed in as {}{}",
                a.email.clone().unwrap_or_else(|| "your Claude account".into()),
                a.plan.as_ref().map(|p| format!(" · {p}")).unwrap_or_default()
            ),
            (crate::brain::BrainState::Starting, None) => "Starting…".into(),
            (crate::brain::BrainState::Ready, None) => "Ready".into(),
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
        let models = [("claude-opus-5-5", "Opus 5.5"), ("claude-sonnet-5-5", "Sonnet 5.5"), ("claude-haiku-5-5", "Haiku 5.5")];
        let mut model_pills = div().mt(px(8.)).flex().gap(px(6.));
        for (i, (id, label)) in models.iter().enumerate() {
            let id = id.to_string();
            model_pills = model_pills.child(pill(("model", i), *label, s.model == id, &t).on_click(cx.listener(move |this, _, window, cx| {
                let v = id.clone();
                this.model.update(cx, |m, cx| m.set_value(v.clone(), window, cx));
                this.update(cx, |s| s.model = v);
            })));
        }
        let effort_row = |label: &'static str, current: String, read: bool, cx: &mut Context<Self>| {
            let mut r = div().mt(px(6.)).flex().items_center().gap(px(6.)).child(div().w(px(110.)).text_size(px(11.5)).text_color(p.graphite).child(label));
            for (i, e) in ["low", "medium", "high"].iter().enumerate() {
                let e = e.to_string();
                let on = current == e;
                r = r.child(
                    pill((if read { "eff-r" } else { "eff-a" }, i), e.clone(), on, &t).on_click(cx.listener(move |this, _, _, cx| {
                        let v = e.clone();
                        this.update(cx, |s| if read { s.effort_read = v } else { s.effort = v });
                    })),
                );
            }
            r
        };
        div()
            .child(div().text_size(px(15.)).text_color(p.ink).child("Model"))
            .child(div().mt(px(4.)).text_size(px(11.5)).text_color(p.graphite).child(
                "Inkling thinks with Claude through the Claude Agent SDK, using this Mac's Claude Code login, so it runs on your Claude subscription. No API key.",
            ))
            .child(
                div()
                    .mt(px(14.))
                    .p(px(10.))
                    .rounded(px(9.))
                    .border_1()
                    .border_color(p.rule)
                    .child(div().flex().items_center().gap(px(7.)).child(ui::small_dot(t.voice_ink, 6.)).child(div().text_size(px(12.5)).text_color(p.ink).child("Claude")))
                    .child(div().mt(px(4.)).text_size(px(11.)).text_color(p.graphite).child(status))
                    .children(limit_line.map(|l| div().mt(px(2.)).text_size(px(11.)).text_color(p.graphite).child(l)))
                    .children(last_error.map(|e| div().mt(px(2.)).text_size(px(11.)).text_color(p.faint).child(format!("Last error: {e}")))),
            )
            .child(div().mt(px(18.)).child(section_label("Model", &t)))
            .child(model_pills)
            .child(
                div()
                    .mt(px(8.))
                    .px(px(8.))
                    .py(px(5.))
                    .rounded(px(7.))
                    .border_1()
                    .border_color(p.rule)
                    .text_size(px(12.))
                    .child(Input::new(&self.model).appearance(false)),
            )
            .child(div().mt(px(18.)).child(section_label("Effort", &t)))
            .child(effort_row("Reading along", s.effort_read.clone(), true, cx))
            .child(effort_row("Asking, drawing", s.effort.clone(), false, cx))
            .child(div().mt(px(5.)).text_size(px(10.5)).text_color(p.faint).child("Reading along runs after every paragraph, so it defaults to low effort."))
            .child(
                div()
                    .mt(px(18.))
                    .p(px(10.))
                    .rounded(px(9.))
                    .border_1()
                    .border_color(p.rule)
                    .child(div().text_size(px(12.5)).text_color(p.ink).child("Codex"))
                    .child(div().mt(px(3.)).text_size(px(11.)).text_color(p.graphite).child("Coming next: sign in with your ChatGPT account using the device code flow, and pick Codex per note.")),
            )
    }

    fn help(&mut self, cx: &mut Context<Self>) -> Div {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let dir = cx.global::<Store>().dir().display().to_string();
        let line = |s: &'static str| div().mt(px(6.)).text_size(px(11.5)).line_height(px(17.)).text_color(p.graphite).child(s);
        div()
            .child(div().text_size(px(15.)).text_color(p.ink).child("Help"))
            .child(line("Write. When a paragraph is done, Inkling reads it and only speaks up when something is worth a second look."))
            .child(line("Select words for the toolbar: Find a loose thread (⌘↵), Ask Inkling, or Sketch it."))
            .child(line("Start a paragraph with “Inkling,” to ask it directly, in either mode."))
            .child(line("⌘K find or start a note · ⌘J chat · ⌘, settings · ⌘⇧. hide threads · ⌘⇧E export"))
            .child(div().mt(px(14.)).text_size(px(10.5)).text_color(p.faint).child(format!("Your notes live in {dir}")))
    }
}

impl Focusable for SettingsView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

fn switch(id: &'static str, on: bool, t: &Theme) -> Stateful<Div> {
    let p = t.p;
    div()
        .id(id)
        .flex_none()
        .w(px(30.))
        .h(px(18.))
        .rounded_full()
        .bg(if on { p.ink } else { p.rule })
        .cursor_pointer()
        .relative()
        .child(
            div()
                .absolute()
                .top(px(2.))
                .left(if on { px(14.) } else { px(2.) })
                .size(px(14.))
                .rounded_full()
                .bg(p.paper),
        )
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let tabs = [
            (Tab::Customize, "Customize Inkling"),
            (Tab::Minds, "Minds"),
            (Tab::AboutYou, "About you"),
            (Tab::Model, "Model"),
            (Tab::Help, "Help"),
        ];
        let current = if self.tab == Tab::NewMind { Tab::Minds } else { self.tab };
        let mut nav = div().w(px(140.)).flex_none().flex().flex_col().gap(px(2.)).p(px(8.)).bg(p.shelf).border_r_1().border_color(p.rule);
        for (i, (tab, label)) in tabs.iter().enumerate() {
            let tab = *tab;
            let on = current == tab;
            nav = nav.child(
                div()
                    .id(("nav", i))
                    .px(px(9.))
                    .py(px(6.))
                    .rounded(px(7.))
                    .text_size(px(12.))
                    .cursor_pointer()
                    .when(on, |d| d.bg(p.paper).border_1().border_color(p.rule).text_color(p.ink))
                    .when(!on, |d| d.text_color(p.graphite).hover(move |s| s.text_color(p.ink)))
                    .child(*label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.tab = tab;
                        cx.notify();
                    })),
            );
        }
        let content = match self.tab {
            Tab::Customize => self.customize(cx),
            Tab::Minds => self.minds(cx),
            Tab::NewMind => self.mind_form(cx),
            Tab::AboutYou => self.about(cx),
            Tab::Model => self.model_tab(cx),
            Tab::Help => self.help(cx),
        };
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
            .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| cx.emit(SettingsEvent::Close)))
            .child(
                div()
                    .id("settings")
                    .track_focus(&self.focus)
                    .w(px(600.))
                    .h(px(520.))
                    .flex()
                    .flex_row()
                    .rounded(px(14.))
                    .overflow_hidden()
                    .bg(p.paper)
                    .border_1()
                    .border_color(p.rule)
                    .font_family(SANS)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_key_down(cx.listener(|_, ev: &KeyDownEvent, _, cx| {
                        if ev.keystroke.key == "escape" {
                            cx.emit(SettingsEvent::Close);
                        }
                    }))
                    .child(nav)
                    .child(
                        div()
                            .w(px(460.))
                            .min_w(px(0.))
                            .flex_none()
                            .h_full()
                            .relative()
                            .overflow_hidden()
                            .child(
                                div()
                                    .id("settings-scroll")
                                    .size_full()
                                    .overflow_y_scroll()
                                    .child(div().w(px(460.)).px(px(22.)).py(px(20.)).child(content.w(px(416.)))),
                            )
                            .child(
                                div()
                                    .id("settings-close")
                                    .absolute()
                                    .top(px(14.))
                                    .right(px(14.))
                                    .cursor_pointer()
                                    .child(ui::icon("x", 14., p.faint))
                                    .on_click(cx.listener(|_, _, _, cx| cx.emit(SettingsEvent::Close))),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

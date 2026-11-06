//! Small shared pieces: icons, buttons, pills, option cards, menus, switches. Sizes come
//! from the type scale in `theme::ts`, and every control has its hover and pressed look
//! here, so the same kind of control behaves the same everywhere.

use gpui_kit::*;
use std::collections::HashMap;
use std::rc::Rc;

use crate::theme::{Palette, SANS, Theme, ts};

// ----- the keyboard ---------------------------------------------------------------------

/// Where each part of the window falls in the Tab order; within a part, its controls follow
/// in the order they are drawn. A part is an id-less `div` with `.tab_group().tab_index(n)`:
/// with no id it gets no focus handle of its own, so a click on it never takes the keyboard
/// away from the note.
pub mod tab {
    // ☰, on the cloth above the sheet, is in no part: it comes first.
    /// The notes list, when it is open.
    pub const SIDEBAR: isize = 2;
    /// The page's header row and the brief under it.
    pub const HEADER: isize = 3;
    /// The note itself.
    pub const NOTE: isize = 4;
    /// The open tag: its reply box, Resolve, Dismiss, Make a note, Copy.
    pub const TAG: isize = 5;
    /// The question box and the selection toolbar.
    pub const ASK: isize = 6;
    /// Chat, and the chat once it is open.
    pub const CHAT: isize = 7;
}

/// Every control that takes the keyboard, by name ("header:mode", "sidebar:note-<id>"), so
/// its focus handle lasts from frame to frame and a dump can say where the keyboard is.
#[derive(Default)]
pub struct Keys(HashMap<SharedString, FocusHandle>);

impl Global for Keys {}

/// The focus handle of the control called `name`: a Tab stop, made on first use.
pub fn key(name: impl Into<SharedString>, cx: &mut App) -> FocusHandle {
    let name = name.into();
    if let Some(h) = cx.try_global::<Keys>().and_then(|k| k.0.get(&name)) {
        return h.clone();
    }
    let h = cx.focus_handle().tab_stop(true);
    cx.default_global::<Keys>().0.insert(name, h.clone());
    h
}

/// The name of the control that has the keyboard, if one of [`key`]'s has it.
pub fn focused_key(window: &Window, cx: &App) -> Option<SharedString> {
    cx.try_global::<Keys>()?.0.iter().find(|(_, h)| h.is_focused(window)).map(|(n, _)| n.clone())
}

/// Return or Space, alone: what presses a control from the keyboard.
pub fn is_press(k: &Keystroke) -> bool {
    (k.key == "enter" || k.key == "space") && !k.modifiers.modified()
}

/// The ring that shows where the keyboard is: a running stitch in `color`, a couple of
/// pixels outside the control it is a child of, drawn while `focus` has the keyboard and
/// the keyboard put it there (a click never leaves one behind).
pub fn focus_ring(focus: &FocusHandle, radius: f32, color: Hsla) -> impl IntoElement {
    let focus = focus.clone();
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            if focus.is_focused(window) && window.last_input_was_keyboard() {
                crate::stitch::focus_stitch(window, bounds, px(radius), color);
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// A control the keyboard can reach: Tab stops on it, Return or Space presses it as a click
/// does, and the stitched ring shows it. A click still works as before and leaves the
/// keyboard where it was, in the note.
pub fn keyed<V: 'static>(
    el: Stateful<Div>,
    focus: &FocusHandle,
    radius: f32,
    ring: Hsla,
    cx: &Context<V>,
    press: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static,
) -> Stateful<Div> {
    let press = Rc::new(press);
    let by_key = press.clone();
    // (no `.relative()`: every element already positions its absolute children, and it would
    // undo a control's own `.absolute()`)
    el.track_focus(focus)
        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| press(this, window, cx)))
        .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, window, cx| {
            if is_press(&ev.keystroke) {
                cx.stop_propagation();
                by_key(this, window, cx);
            }
        }))
        .child(focus_ring(focus, radius, ring))
}

pub fn icon(name: &str, size: f32, color: Hsla) -> Svg {
    svg()
        .path(SharedString::from(format!("icons/{name}.svg")))
        .size(px(size))
        .flex_none()
        .text_color(color)
}

/// An icon you can click: a quiet glyph with a soft square behind it under the pointer
/// (close buttons, the sketch's step controls).
pub fn icon_button(id: impl Into<ElementId>, name: &str, size: f32, t: &Theme) -> Stateful<Div> {
    let p = t.p;
    div()
        .id(id)
        .flex_none()
        .p(px(4.))
        .rounded(px(5.))
        .cursor_pointer()
        .hover(move |s| s.bg(p.hover))
        .active(|s| s.opacity(0.7))
        .child(icon(name, size, p.graphite))
}

/// A rounded chip, filled when selected (mode and template chips).
pub fn pill(id: impl Into<ElementId>, label: impl Into<SharedString>, selected: bool, t: &Theme) -> Stateful<Div> {
    let p = t.p;
    div()
        .id(id)
        .flex_none()
        .px(px(10.))
        .py(px(4.))
        .rounded_full()
        .text_size(px(ts::SMALL))
        .border_1()
        .border_color(if selected { p.ink } else { p.rule })
        .bg(if selected { p.ink } else { p.paper })
        .text_color(if selected { p.paper } else { p.graphite })
        .cursor_pointer()
        .hover(move |s| if selected { s } else { s.border_color(p.faint).text_color(p.ink) })
        .active(|s| s.opacity(0.8))
        .child(label.into())
}

/// A dark, rounded primary button.
pub fn primary_button(id: impl Into<ElementId>, label: impl Into<SharedString>, t: &Theme) -> Stateful<Div> {
    let p = t.p;
    div()
        .id(id)
        .flex_none()
        .px(px(12.))
        .py(px(5.))
        .rounded_full()
        .bg(p.ink)
        .text_color(p.paper)
        .text_size(px(ts::UI))
        .cursor_pointer()
        .hover(move |s| s.opacity(0.88))
        .active(|s| s.opacity(0.75))
        .child(label.into())
}

/// Quiet text button, graphite until hovered.
pub fn ghost_button(id: impl Into<ElementId>, label: impl Into<SharedString>, t: &Theme) -> Stateful<Div> {
    let p = t.p;
    div()
        .id(id)
        .flex_none()
        .text_size(px(ts::SMALL))
        .text_color(p.faint)
        .cursor_pointer()
        .hover(move |s| s.text_color(p.ink))
        .active(|s| s.opacity(0.7))
        .child(label.into())
}

/// A ghost button that cannot be pressed right now: the same words, dimmed, inert.
pub fn ghost_button_off(label: impl Into<SharedString>, t: &Theme) -> Div {
    div().flex_none().text_size(px(ts::SMALL)).text_color(t.p.faint).opacity(0.45).child(label.into())
}

/// A selectable card with a title and a short description (settings choices).
pub fn option_card(
    id: impl Into<ElementId>,
    title: impl Into<SharedString>,
    body: impl Into<SharedString>,
    selected: bool,
    t: &Theme,
) -> Stateful<Div> {
    let p = t.p;
    div()
        .id(id)
        .flex_1()
        .min_w(px(0.))
        .px(px(10.))
        .py(px(9.))
        .rounded(px(9.))
        .border_1()
        .border_color(if selected { p.ink } else { p.rule })
        .cursor_pointer()
        // half way to the faint ink: a card under the pointer must not look chosen
        .hover(move |s| if selected { s } else { s.border_color(p.faint.opacity(0.5)) })
        .active(|s| s.opacity(0.85))
        .child(div().text_size(px(ts::UI)).text_color(p.ink).child(title.into()))
        .child(div().mt(px(3.)).text_size(px(ts::META)).line_height(px(15.)).text_color(p.graphite).child(body.into()))
}

/// An on/off switch.
pub fn switch(id: impl Into<ElementId>, on: bool, t: &Theme) -> Stateful<Div> {
    let p = t.p;
    div()
        .id(id)
        .flex_none()
        .w(px(30.))
        .h(px(18.))
        .rounded_full()
        .relative()
        .bg(if on { p.ink } else { p.rule })
        .cursor_pointer()
        .hover(move |s| if on { s.opacity(0.88) } else { s.bg(p.faint.opacity(0.5)) })
        .child(div().absolute().top(px(2.)).left(if on { px(14.) } else { px(2.) }).size(px(14.)).rounded_full().bg(p.paper))
}

/// [`keyed`] in a builder chain, with the control's focus handle named `name` (see [`key`]).
pub trait Keyed: Sized {
    fn keyed_as<V: 'static>(
        self,
        name: impl Into<SharedString>,
        radius: f32,
        ring: Hsla,
        cx: &mut Context<V>,
        press: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static,
    ) -> Self;
}

impl Keyed for Stateful<Div> {
    fn keyed_as<V: 'static>(
        self,
        name: impl Into<SharedString>,
        radius: f32,
        ring: Hsla,
        cx: &mut Context<V>,
        press: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static,
    ) -> Self {
        let focus = key(name, cx);
        keyed(self, &focus, radius, ring, cx, press)
    }
}

/// The shadow under menus and popovers: lifted a little off the page.
pub fn popover_shadow(p: &Palette) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: hsla(0., 0., 0., if p.dark { 0.45 } else { 0.10 }),
        offset: point(px(0.), px(8.)),
        blur_radius: px(24.),
        spread_radius: px(-6.),
        inset: false,
    }]
}

/// The shadow under dialogs (the palette, export, a sketch seen larger): further off.
pub fn modal_shadow(p: &Palette) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: hsla(0., 0., 0., if p.dark { 0.5 } else { 0.14 }),
        offset: point(px(0.), px(12.)),
        blur_radius: px(32.),
        spread_radius: px(-8.),
        inset: false,
    }]
}

/// A floating card used for menus and popovers.
pub fn menu_card(t: &Theme) -> Div {
    let p = t.p;
    div()
        .flex()
        .flex_col()
        .py(px(5.))
        .rounded(px(10.))
        .bg(p.card)
        .border_1()
        .border_color(p.rule)
        .shadow(popover_shadow(&p))
        .text_size(px(ts::UI))
        .font_family(SANS)
}

pub fn menu_item(id: impl Into<ElementId>, t: &Theme) -> Stateful<Div> {
    let p = t.p;
    div()
        .id(id)
        .mx(px(5.))
        .px(px(9.))
        .py(px(6.))
        .rounded(px(6.))
        .cursor_pointer()
        .text_color(p.ink)
        .hover(move |s| s.bg(p.hover))
        .active(|s| s.opacity(0.8))
}

pub fn section_label(text: impl Into<SharedString>, t: &Theme) -> Div {
    div()
        .text_size(px(ts::UI))
        .text_color(t.p.ink)
        .font_weight(FontWeight::MEDIUM)
        .child(text.into())
}

/// A keyboard shortcut beside the thing it does, in the quietest text.
pub fn key_hint(keys: impl Into<SharedString>, t: &Theme) -> Div {
    div().flex_none().text_size(px(ts::META)).text_color(t.p.faint).child(keys.into())
}

pub fn small_dot(color: Hsla, size: f32) -> Div {
    div().flex_none().size(px(size)).rounded_full().bg(color)
}

/// The wordmark: "inkling" in Fraunces Display, its first i in your thread and dotted
/// with a French knot, as the Initial mark marks the app the way linen was marked as
/// someone's own. The smallest piece of ink in the name is where Inkling lives.
pub fn wordmark(t: &Theme, size: f32) -> impl IntoElement {
    let ink = t.p.ink;
    let thread = t.voice_ink;
    let h = size * 1.3;
    canvas(
        |_, _, _| (),
        move |bounds, _, window, cx| {
            let text: SharedString = "\u{131}nkling".into();
            let first = "\u{131}".len();
            let run = |len: usize, color: Hsla| TextRun {
                len,
                font: Font {
                    family: crate::theme::DISPLAY.into(),
                    features: FontFeatures::default(),
                    fallbacks: None,
                    weight: FontWeight::MEDIUM,
                    style: FontStyle::Normal,
                },
                color,
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            // the initial in your thread, the rest of the name in ink
            let runs = [run(first, thread), run(text.len() - first, ink)];
            let shaped = window.text_system().shape_line(text, px(size), &runs, None);
            let _ = shaped.paint(bounds.origin, px(h), TextAlign::Left, None, window, cx);
            // the dotless i is the first glyph; its stem sits a little left of centre
            let i_w = shaped.x_for_index("\u{131}".len());
            let x = bounds.origin.x + i_w * 0.5;
            // the knot sits as close over the stem as the Initial mark's does: a gap of about
            // a tenth of the size, the knot a little wider than the stem
            let y = bounds.origin.y + px(h * 0.5 - size * 0.48);
            // the dot is a French knot in the same thread, as in the Initial mark
            crate::stitch::french_knot(window, point(x, y), px(size * 0.12), thread, 1.);
        },
    )
    .w(px(size * 3.25))
    .h(px(h))
}

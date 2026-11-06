//! Small shared pieces: icons, buttons, pills, option cards, menus, switches. Sizes come
//! from the type scale in `theme::ts`, and every control has its hover and pressed look
//! here, so the same kind of control behaves the same everywhere.

use gpui_kit::*;
use gpui_kit::prelude::*;

use crate::theme::{Palette, SANS, Theme, ts};

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
        .hover(move |s| s.bg(p.wash))
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
        .hover(move |s| if selected { s } else { s.border_color(p.faint) })
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
        .hover(move |s| s.bg(p.wash))
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

/// The wordmark: "inkling" in Fraunces Display with a French knot for the dot of the
/// first i. The smallest piece of ink in the name is where Inkling lives.
pub fn wordmark(t: &Theme, size: f32) -> impl IntoElement {
    let ink = t.p.ink;
    let thread = t.voice_ink;
    let h = size * 1.3;
    canvas(
        |_, _, _| (),
        move |bounds, _, window, cx| {
            let text: SharedString = "\u{131}nkling".into();
            let run = TextRun {
                len: text.len(),
                font: Font {
                    family: crate::theme::DISPLAY.into(),
                    features: FontFeatures::default(),
                    fallbacks: None,
                    weight: FontWeight::MEDIUM,
                    style: FontStyle::Normal,
                },
                color: ink,
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let shaped = window.text_system().shape_line(text, px(size), &[run], None);
            let _ = shaped.paint(bounds.origin, px(h), TextAlign::Left, None, window, cx);
            // the dotless i is the first glyph; its stem sits a little left of centre
            let i_w = shaped.x_for_index("\u{131}".len());
            let x = bounds.origin.x + i_w * 0.47;
            let y = bounds.origin.y + px(h * 0.5 - size * 0.6);
            // the dot is one cross-stitch in Inkling's thread, as in the Darn · Border wordmark
            crate::stitch::cross_stitch(window, point(x, y), px(size * 0.085), thread);
        },
    )
    .w(px(size * 3.25))
    .h(px(h))
}

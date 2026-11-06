//! Small shared pieces: icons, pills, option cards, menus.

use gpui_kit::*;
use gpui_kit::prelude::*;

use crate::theme::{SANS, Theme};

pub fn icon(name: &str, size: f32, color: Hsla) -> Svg {
    svg()
        .path(SharedString::from(format!("icons/{name}.svg")))
        .size(px(size))
        .flex_none()
        .text_color(color)
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
        .text_size(px(12.))
        .border_1()
        .border_color(if selected { p.ink } else { p.rule })
        .bg(if selected { p.ink } else { p.paper })
        .text_color(if selected { p.paper } else { p.graphite })
        .cursor_pointer()
        .hover(move |s| if selected { s } else { s.border_color(p.faint).text_color(p.ink) })
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
        .text_size(px(12.5))
        .cursor_pointer()
        .hover(move |s| s.opacity(0.88))
        .child(label.into())
}

/// Quiet text button, graphite until hovered.
pub fn ghost_button(id: impl Into<ElementId>, label: impl Into<SharedString>, t: &Theme) -> Stateful<Div> {
    let p = t.p;
    div()
        .id(id)
        .flex_none()
        .text_size(px(12.))
        .text_color(p.faint)
        .cursor_pointer()
        .hover(move |s| s.text_color(p.ink))
        .child(label.into())
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
        .child(div().text_size(px(12.5)).text_color(p.ink).child(title.into()))
        .child(div().mt(px(3.)).text_size(px(11.)).line_height(px(15.)).text_color(p.graphite).child(body.into()))
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
        .shadow(vec![BoxShadow {
            color: hsla(0., 0., 0., if p.dark { 0.45 } else { 0.10 }),
            offset: point(px(0.), px(8.)),
            blur_radius: px(24.),
            spread_radius: px(-6.),
            inset: false,
        }])
        .text_size(px(12.5))
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
}

pub fn section_label(text: impl Into<SharedString>, t: &Theme) -> Div {
    div()
        .text_size(px(12.5))
        .text_color(t.p.ink)
        .font_weight(FontWeight::MEDIUM)
        .child(text.into())
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
            crate::stitch::french_knot(window, point(x, y), px(size * 0.13), thread, 1.);
        },
    )
    .w(px(size * 3.25))
    .h(px(h))
}

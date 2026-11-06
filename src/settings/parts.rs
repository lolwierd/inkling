//! What Settings is built from: its glass, controls drawn the way a Mac draws them in
//! Inkling's paper, ink and thread, and the small pictures that show a choice as the
//! page it makes.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::store::Placement;
use crate::theme::{Palette, Theme};
use crate::ui::icon;

/// The window's materials. The desk is linen with the desktop showing through it,
/// blurred; the sidebar is frosted glass resting on the desk; the page is flat paper,
/// because words sit on flat paper. How much shows through the sidebar is set so its text
/// keeps 4.5:1 over a black or a white desktop (see the tests).
#[derive(Clone, Copy, Debug)]
pub struct Glass {
    pub desk: Hsla,
    /// The sidebar's frost, lighter at the top where the light falls.
    pub pane_top: Hsla,
    pub pane_bottom: Hsla,
    /// The pane's edge catching the light, and the highlight along its top.
    pub rim: Hsla,
    pub sheen: Hsla,
    /// Shadow under anything that floats (one light, from above).
    pub lift: Hsla,
    /// The sidebar row you are on.
    pub chosen: Hsla,
    pub chosen_rim: Hsla,
    pub hover: Hsla,
    pub page: Hsla,
    pub page_rim: Hsla,
    /// Grouped rows on the page.
    pub group: Hsla,
    /// The groove a segmented control's choices sit in.
    pub track: Hsla,
}

fn white(a: f32) -> Hsla {
    hsla(0., 0., 1., a)
}

impl Glass {
    /// How opaque the desk and the bottom of the sidebar are, in each light: the
    /// sidebar's text sits on both.
    pub const LIGHT: (f32, f32) = (0.62, 0.55);
    pub const DARK: (f32, f32) = (0.72, 0.48);

    pub fn of(p: &Palette) -> Glass {
        if p.dark {
            let (desk, pane) = Self::DARK;
            Glass {
                desk: p.cloth.opacity(desk),
                pane_top: p.card.opacity(pane + 0.08),
                pane_bottom: p.card.opacity(pane),
                rim: white(0.10),
                sheen: white(0.07),
                lift: hsla(0., 0., 0., 0.45),
                chosen: white(0.09),
                chosen_rim: white(0.07),
                hover: white(0.05),
                page: p.paper,
                page_rim: p.rule,
                group: p.card,
                track: white(0.07),
            }
        } else {
            let (desk, pane) = Self::LIGHT;
            Glass {
                desk: p.cloth.opacity(desk),
                pane_top: p.paper.opacity(pane + 0.11),
                pane_bottom: p.paper.opacity(pane),
                rim: white(0.8),
                sheen: white(0.9),
                lift: hsla(30. / 360., 0.5, 0.16, 0.12),
                chosen: white(0.8),
                chosen_rim: p.rule.opacity(0.8),
                hover: p.ink.opacity(0.05),
                page: p.paper,
                page_rim: p.rule,
                group: p.card,
                track: p.ink.opacity(0.06),
            }
        }
    }

    pub fn float(&self, y: f32, blur: f32) -> Vec<BoxShadow> {
        vec![BoxShadow { color: self.lift, offset: point(px(0.), px(y)), blur_radius: px(blur), spread_radius: px(0.), inset: false }]
    }

    /// The inner edge of thick glass: a hairline of light just inside the rim.
    pub fn inner_edge(&self, radius: f32) -> Div {
        div().absolute().top(px(1.)).left(px(1.)).right(px(1.)).bottom(px(1.)).rounded(px(radius - 1.)).border_1().border_color(self.sheen.opacity(0.5))
    }

    /// A floating pane's shadow plus the highlight along its top edge.
    pub fn floating(&self) -> Vec<BoxShadow> {
        let mut s = self.float(8., 24.);
        s.push(BoxShadow { color: self.sheen, offset: point(px(0.), px(1.)), blur_radius: px(0.), spread_radius: px(0.), inset: true });
        s
    }
}

/// A heading above a group, with a quiet note at its right.
pub fn section(title: impl Into<SharedString>, aside: Option<SharedString>, t: &Theme) -> Div {
    let p = t.p;
    div()
        .mt(px(24.))
        .mb(px(8.))
        .px(px(2.))
        .flex()
        .items_baseline()
        .gap(px(12.))
        .child(crate::ui::section_label(title, t).flex_1().text_size(px(12.)))
        .children(aside.map(|a| div().text_size(px(11.)).text_color(p.faint).child(a)))
}

/// Rows grouped on a card, with hairlines between them inset from the left, as macOS does.
pub fn group(rows: Vec<AnyElement>, t: &Theme) -> Div {
    let g = Glass::of(&t.p);
    let mut out = div().w_full().flex().flex_col().rounded(px(10.)).bg(g.group).border_1().border_color(t.p.rule);
    for (i, r) in rows.into_iter().enumerate() {
        if i > 0 {
            out = out.child(div().h(px(1.)).ml(px(14.)).bg(t.p.rule));
        }
        out = out.child(r);
    }
    out
}

/// A row: a title and a line about it on the left, its control on the right.
pub fn row(title: impl IntoElement, detail: Option<SharedString>, control: impl IntoElement, t: &Theme) -> Div {
    let p = t.p;
    div()
        .min_h(px(44.))
        .px(px(14.))
        .py(px(10.))
        .flex()
        .items_center()
        .gap(px(16.))
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(div().text_size(px(13.)).text_color(p.ink).child(title))
                .children(detail.map(|d| div().text_size(px(11.5)).line_height(px(16.)).text_color(p.graphite).child(d))),
        )
        .child(control)
}

/// A line of small print under a group.
pub fn note(text: impl Into<SharedString>, t: &Theme) -> Div {
    div().mt(px(7.)).px(px(2.)).text_size(px(11.)).line_height(px(15.)).text_color(t.p.faint).child(text.into())
}

/// The groove a segmented control's choices sit in.
pub fn segmented(t: &Theme) -> Div {
    let g = Glass::of(&t.p);
    div().flex_none().flex().p(px(2.)).gap(px(2.)).rounded(px(8.)).bg(g.track)
}

/// One choice of a segmented control; the chosen one is raised out of the groove.
pub fn segment(id: impl Into<ElementId>, label: impl Into<SharedString>, on: bool, t: &Theme) -> Stateful<Div> {
    let p = t.p;
    let g = Glass::of(&p);
    div()
        .id(id)
        .h(px(24.))
        .px(px(12.))
        .rounded(px(6.))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(12.))
        .cursor_pointer()
        .when(on, |d| d.bg(p.card).text_color(p.ink).font_weight(FontWeight::MEDIUM).shadow(g.float(1., 2.)))
        .when(!on, |d| d.text_color(p.graphite).hover(move |s| s.text_color(p.ink)).active(move |s| s.bg(g.hover)))
        .child(label.into())
}

/// An on/off switch: ink when on, as everywhere in Inkling.
pub fn switch(id: impl Into<ElementId>, on: bool, t: &Theme) -> Stateful<Div> {
    let p = t.p;
    let g = Glass::of(&p);
    let off_knob = if p.dark { p.graphite } else { white(1.) };
    div()
        .id(id)
        .flex_none()
        .w(px(34.))
        .h(px(20.))
        .rounded_full()
        .relative()
        .cursor_pointer()
        .bg(if on { p.ink } else { p.ink.opacity(if p.dark { 0.22 } else { 0.16 }) })
        .child(
            div()
                .absolute()
                .top(px(2.))
                .left(if on { px(16.) } else { px(2.) })
                .size(px(16.))
                .rounded_full()
                .bg(if on { p.paper } else { off_knob })
                .shadow(g.float(1., 2.)),
        )
}

/// A button with a border, as macOS draws push buttons.
pub fn push_button(id: impl Into<ElementId>, label: impl Into<SharedString>, t: &Theme) -> Stateful<Div> {
    let p = t.p;
    let g = Glass::of(&p);
    div()
        .id(id)
        .flex_none()
        .h(px(26.))
        .px(px(12.))
        .flex()
        .items_center()
        .rounded(px(7.))
        .bg(p.card)
        .border_1()
        .border_color(p.rule)
        .shadow(g.float(1., 1.))
        .text_size(px(12.))
        .text_color(p.ink)
        .cursor_pointer()
        .hover(move |s| s.border_color(p.faint))
        .active(move |s| s.bg(g.hover))
        .child(label.into())
}

/// The one button on a page that finishes the job: filled with ink.
pub fn default_button(id: impl Into<ElementId>, label: impl Into<SharedString>, enabled: bool, t: &Theme) -> Stateful<Div> {
    let p = t.p;
    div()
        .id(id)
        .flex_none()
        .h(px(26.))
        .px(px(14.))
        .flex()
        .items_center()
        .rounded(px(7.))
        .bg(p.ink)
        .text_size(px(12.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(p.paper)
        .when(enabled, |d| d.cursor_pointer().hover(|s| s.opacity(0.9)).active(|s| s.opacity(0.8)))
        .when(!enabled, |d| d.opacity(0.4).cursor_default())
        .child(label.into())
}

/// Quiet text that acts: graphite until pointed at.
pub fn text_button(id: impl Into<ElementId>, label: impl Into<SharedString>, t: &Theme) -> Stateful<Div> {
    let p = t.p;
    div()
        .id(id)
        .flex_none()
        .text_size(px(12.))
        .text_color(p.graphite)
        .cursor_pointer()
        .hover(move |s| s.text_color(p.ink))
        .child(label.into())
}

/// A card to choose; the chosen one is outlined in ink and ticked, so it reads without
/// colour. The tick goes on after `body`, so it sits on top of a picture.
pub fn tile(id: impl Into<ElementId>, on: bool, t: &Theme, body: impl IntoElement) -> Stateful<Div> {
    let p = t.p;
    div()
        .id(id)
        .flex_1()
        .min_w(px(0.))
        .relative()
        .rounded(px(10.))
        .bg(p.card)
        .border_1()
        .border_color(if on { p.ink } else { p.rule })
        .cursor_pointer()
        .when(!on, |d| d.hover(move |s| s.border_color(p.faint)))
        .child(body)
        .when(on, |d| d.child(tick(t)))
}

/// The tick in a chosen tile's corner.
fn tick(t: &Theme) -> Div {
    let p = t.p;
    div()
        .absolute()
        .top(px(7.))
        .right(px(7.))
        .size(px(16.))
        .rounded_full()
        .bg(p.ink)
        .flex()
        .items_center()
        .justify_center()
        .child(icon("check", 10., p.paper))
}

/// How a shortcut's keycap is drawn.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Cap {
    Keys,
    /// Its keys also run something else where both work.
    Clash,
    /// Listening for new keys.
    Listening,
    /// No keys.
    Empty,
}

pub fn keycap(id: impl Into<ElementId>, text: impl Into<SharedString>, cap: Cap, warn: Hsla, t: &Theme) -> Stateful<Div> {
    let p = t.p;
    let g = Glass::of(&p);
    div()
        .id(id)
        .flex_none()
        .h(px(24.))
        .min_w(px(30.))
        .px(px(8.))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(5.))
        .rounded(px(6.))
        .text_size(px(12.))
        .cursor_pointer()
        .border_1()
        .map(|d| match cap {
            Cap::Keys => d.bg(p.card).border_color(p.rule).text_color(p.ink).shadow(g.float(1., 0.)),
            Cap::Clash => d.bg(p.card).border_color(warn).text_color(p.ink).shadow(g.float(1., 0.)),
            Cap::Listening => d.bg(p.paper).border_color(t.voice_ink).text_color(p.graphite),
            Cap::Empty => d.border_color(p.rule).text_color(p.faint),
        })
        .when(cap != Cap::Listening, |d| d.hover(move |s| s.border_color(p.faint)))
        .child(text.into())
}

// ----- pictures of a choice ------------------------------------------------------------

fn bar(x: f32, y: f32, w: f32, color: Hsla) -> Div {
    div().absolute().left(px(x)).top(px(y)).w(px(w)).h(px(2.)).rounded_full().bg(color)
}

/// A running stitch: short dashes in the thread.
fn stitches(x: f32, y: f32, w: f32, color: Hsla) -> Div {
    let mut d = div().absolute().left(px(x)).top(px(y)).w(px(w)).h(px(2.)).flex().gap(px(2.));
    let n = ((w + 2.) / 6.).floor().max(1.) as usize;
    for _ in 0..n {
        d = d.child(div().w(px(4.)).h(px(1.5)).rounded_full().bg(color));
    }
    d
}

/// A fabric tag in the margin with its thread running to the words.
fn tag(x: f32, y: f32, thread_from: f32, p: &Palette, thread: Hsla) -> Vec<Div> {
    let (w, h) = (24., 13.);
    let (lo, hi) = if thread_from < x { (thread_from, x) } else { (x + w, thread_from) };
    vec![
        div().absolute().left(px(lo)).top(px(y + h / 2.)).w(px(hi - lo)).h(px(1.)).bg(thread.opacity(0.55)),
        div()
            .absolute()
            .left(px(x))
            .top(px(y))
            .w(px(w))
            .h(px(h))
            .rounded(px(3.))
            .bg(p.card)
            .border_1()
            .border_color(thread.opacity(0.7))
            .child(div().absolute().left(px(4.)).top(px(4.)).w(px(14.)).h(px(1.5)).bg(p.ink.opacity(0.3)))
            .child(div().absolute().left(px(4.)).top(px(7.5)).w(px(9.)).h(px(1.5)).bg(p.ink.opacity(0.3))),
    ]
}

/// A page a few centimetres wide: linen around a flat sheet, lines of words, a phrase
/// stitched in the thread, and its question where `placement` puts it.
pub fn page_picture(placement: Placement, p: &Palette, thread: Hsla) -> Div {
    let (w, h) = (152., 86.);
    let (sheet_x, sheet_w) = (40., 72.);
    let ink = p.ink.opacity(0.22);
    let mut d = div()
        .w(px(w))
        .h(px(h))
        .flex_none()
        .relative()
        .overflow_hidden()
        .rounded(px(7.))
        .bg(p.cloth)
        .child(div().absolute().left(px(sheet_x)).top(px(10.)).w(px(sheet_w)).h(px(h)).rounded(px(3.)).bg(p.paper));
    let lines = [(18., 56.), (27., 60.), (36., 50.), (45., 58.), (54., 40.), (63., 56.), (72., 48.)];
    for (y, lw) in lines {
        d = d.child(bar(sheet_x + 8., y, lw, ink));
    }
    // The stitched phrase on the second line, and a second on the fifth.
    let first = (sheet_x + 20., 31., 30.);
    let second = (sheet_x + 8., 58., 26.);
    d = d.child(stitches(first.0, first.1, first.2, thread)).child(stitches(second.0, second.1, second.2, thread));
    match placement {
        Placement::Both => {
            d = d.children(tag(8., 25., sheet_x + 20., p, thread));
            d = d.children(tag(w - 32., 52., sheet_x + 8. + 26., p, thread));
        }
        Placement::Right => {
            d = d.children(tag(w - 32., 25., first.0 + first.2, p, thread));
            d = d.children(tag(w - 32., 52., second.0 + second.2, p, thread));
        }
        Placement::Marks => {
            // the ? the page puts after a passage
            for (x, y) in [(first.0 + first.2 + 2., first.1 - 7.), (second.0 + second.2 + 2., second.1 - 7.)] {
                d = d.child(
                    div().absolute().left(px(x)).top(px(y)).text_size(px(9.)).line_height(px(10.)).font_weight(FontWeight::BOLD).text_color(thread).child("?"),
                );
            }
        }
    }
    d
}

/// A window a few centimetres wide, in linen or lamplight, with a question in its margin.
pub fn window_picture(p: &Palette, thread: Hsla) -> Div {
    let (w, h) = (152., 86.);
    let ink = p.ink.opacity(0.24);
    let mut d = div()
        .w(px(w))
        .h(px(h))
        .flex_none()
        .relative()
        .overflow_hidden()
        .rounded(px(7.))
        .bg(p.cloth)
        .child(div().absolute().left_0().top_0().w(px(w)).h(px(14.)).bg(p.shelf));
    for i in 0..3 {
        d = d.child(div().absolute().left(px(7. + i as f32 * 7.)).top(px(5.)).size(px(4.)).rounded_full().bg(p.ink.opacity(0.28)));
    }
    d = d.child(div().absolute().left(px(30.)).top(px(22.)).w(px(76.)).h(px(h)).rounded(px(3.)).bg(p.paper));
    for (i, lw) in [50., 60., 42., 56., 48.].into_iter().enumerate() {
        d = d.child(bar(38., 31. + i as f32 * 9., lw, ink));
    }
    d = d.child(stitches(52., 44., 28., thread));
    d.children(tag(w - 34., 37., 80., p, thread))
}

/// The light and the dark picture side by side, split down the middle: "follows macOS".
pub fn split_picture(light: Div, dark: Div) -> Div {
    let (w, h) = (152., 86.);
    div()
        .w(px(w))
        .h(px(h))
        .flex_none()
        .relative()
        .overflow_hidden()
        .rounded(px(7.))
        .child(div().absolute().left_0().top_0().child(light))
        .child(div().absolute().left(px(w / 2.)).top_0().w(px(w / 2.)).h(px(h)).overflow_hidden().child(div().absolute().left(px(-w / 2.)).top_0().child(dark)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Palette;

    /// WCAG contrast of two opaque colours.
    fn contrast(a: Rgba, b: Rgba) -> f32 {
        let lum = |c: Rgba| {
            let f = |v: f32| if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
            0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.0722 * f(c.b)
        };
        let (x, y) = (lum(a), lum(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    fn over(top: Hsla, under: Rgba) -> Rgba {
        let c = top.to_rgb();
        let a = top.a;
        Rgba { r: c.r * a + under.r * (1. - a), g: c.g * a + under.g * (1. - a), b: c.b * a + under.b * (1. - a), a: 1. }
    }

    #[std::prelude::v1::test]
    fn sidebar_text_reads_over_any_desktop() {
        for p in [Palette::light(), Palette::dark()] {
            let g = Glass::of(&p);
            for desktop in [Rgba { r: 0., g: 0., b: 0., a: 1. }, Rgba { r: 1., g: 1., b: 1., a: 1. }] {
                // Where the sidebar is clearest: its bottom, on the desk.
                let behind = over(g.pane_bottom, over(g.desk, desktop));
                for (name, fg) in [("ink", p.ink), ("graphite", p.graphite)] {
                    let c = contrast(fg.to_rgb(), behind);
                    assert!(c >= 4.5, "{name} on the {} sidebar over {desktop:?}: {c:.2}", if p.dark { "dark" } else { "light" });
                }
                // The selected row is lighter still, and its text is ink.
                let chosen = over(g.chosen, behind);
                assert!(contrast(p.ink.to_rgb(), chosen) >= 4.5);
            }
        }
    }
}

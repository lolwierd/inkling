//! Colours, fonts and sizes for Loose Threads: an embroidery sampler in its hoop.
//! Linen around a flat page, your words in Fraunces, Inkling's in Figtree, and every
//! mind its own thread colour. Texture lives on the cloth around the page, never
//! under the words.

use gpui_kit::{App, Global, Hsla, Pixels, Rgba, SharedString, WindowAppearance, px, rgb, rgba};
use std::borrow::Cow;

use crate::store::{Appearance, InkColor, Lettering, TextSize};

/// The interface and Inkling's voice: round, clear, no costume.
pub const SANS: &str = "Figtree";
/// The writer's words: Fraunces at its text optical size, with the soft axis at 100 so
/// the corners round off the way thread rounds a letter.
pub const SERIF: &str = "Fraunces Text";
/// Titles and headings: Fraunces at its display optical size.
pub const DISPLAY: &str = "Fraunces Display";
/// Code blocks: Spline Sans Mono, whose weight on the page matches Fraunces Text, so a
/// block of code is neither a grey hole nor a dark slab.
pub const MONO: &str = "Spline Sans Mono";
/// Inline code: the same face drawn at 0.86 of the size it is set at
/// (`design/tools/fetch-app-fonts.py`), because a line of text has one size.
pub const MONO_INLINE: &str = "Spline Sans Mono Inline";

/// The interface's type scale, in Figtree: five sizes, so labels, rows and hints line up
/// from one surface to the next. The writer's words follow the text-size setting
/// ([`note_size`]) instead.
pub mod ts {
    /// Key hints, counts, captions and fine print.
    pub const META: f32 = 11.;
    /// Secondary lines and quiet buttons.
    pub const SMALL: f32 = 12.;
    /// Menus, rows, labels and buttons: the interface's own size.
    pub const UI: f32 = 13.;
    /// Text fields, and Inkling's words in a panel.
    pub const BODY: f32 = 14.;
    /// Dialog and panel titles.
    pub const TITLE: f32 = 15.;
}

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    /// The writing page: flat, no texture.
    pub paper: Hsla,
    /// Sidebar and panels.
    pub shelf: Hsla,
    /// The linen around the page (the desk); the only place the weave shows.
    pub cloth: Hsla,
    /// The weave lines drawn on the cloth.
    pub weave: Hsla,
    /// The ground under code: a step darker than the page in linen, a step lighter in
    /// lamplight. Flat, with no edge.
    pub code: Hsla,
    /// Your words.
    pub ink: Hsla,
    /// Secondary text.
    pub graphite: Hsla,
    /// Labels and quiet text (still 4.5:1 on the page).
    pub faint: Hsla,
    pub rule: Hsla,
    /// Behind the words of the thread you are looking at.
    pub wash: Hsla,
    /// Tacking thread: a done checklist line is tacked through with it, and a quote's
    /// seam is sewn in it.
    pub tack: Hsla,
    /// Busy marks (the needle while reading).
    pub bulb: Hsla,
    /// Plain lines that are not thread.
    pub line: Hsla,
    pub selection: Hsla,
    pub dim: Hsla,
    /// Fabric tags in the margin.
    pub card: Hsla,
    /// Soft shadow under thread and tags (one light, from the top left).
    pub shadow: Hsla,
    pub dark: bool,
}

fn c(hex: u32) -> Hsla {
    rgb(hex).into()
}

fn ca(hex: u32) -> Hsla {
    let r: Rgba = rgba(hex);
    r.into()
}

impl Palette {
    /// "Linen": unbleached cloth around a cream page.
    pub fn light() -> Self {
        Self {
            paper: c(0xfbf8f2),
            shelf: c(0xf3ede2),
            cloth: c(0xefe8db),
            weave: ca(0x5a401e14),
            code: c(0xf1eadc),
            ink: c(0x26232b),
            graphite: c(0x57505c),
            faint: c(0x6f6862),
            rule: c(0xe3dacb),
            wash: ca(0xa63f3214),
            tack: c(0x8c8276),
            bulb: c(0x57505c),
            line: ca(0x26232b4d),
            selection: ca(0x2e3f7a2e),
            dim: ca(0x26232b40),
            card: c(0xfffdf8),
            shadow: ca(0x3c281424),
            dark: false,
        }
    }

    /// "Lamplight": the same cloth after dark, warm grey under a lamp.
    pub fn dark() -> Self {
        Self {
            paper: c(0x1f1d22),
            shelf: c(0x1a181d),
            cloth: c(0x18161a),
            weave: ca(0xffecd20a),
            code: c(0x2a272e),
            ink: c(0xede6da),
            graphite: c(0xbdb4a8),
            faint: c(0xa39a8f),
            rule: c(0x332f36),
            wash: ca(0xd9705e1f),
            tack: c(0x948a80),
            bulb: c(0xbdb4a8),
            line: ca(0xede6da4d),
            selection: ca(0x8c9be047),
            dim: ca(0x00000080),
            card: c(0x26232a),
            shadow: ca(0x00000059),
            dark: true,
        }
    }
}

/// Everything views need to draw: palette plus the user's display choices.
#[derive(Clone, Debug)]
pub struct Theme {
    pub p: Palette,
    /// The mind's thread colour: its stitches, its thread, the label on its tags.
    pub voice_ink: Hsla,
    /// Typeface for the mind's own words.
    pub voice_font: SharedString,
    pub note_size: Pixels,
}

impl Global for Theme {}

impl Theme {
    pub fn resolve(
        appearance: Appearance,
        window_appearance: WindowAppearance,
        ink: InkColor,
        lettering: Lettering,
        size: TextSize,
    ) -> Self {
        let dark = match appearance {
            Appearance::Light => false,
            Appearance::Dark => true,
            Appearance::System => matches!(
                window_appearance,
                WindowAppearance::Dark | WindowAppearance::VibrantDark
            ),
        };
        let p = if dark { Palette::dark() } else { Palette::light() };
        Self {
            voice_ink: ink_color(ink, &p),
            voice_font: match lettering {
                Lettering::Own => SANS.into(),
                Lettering::LikeNotes => SERIF.into(),
            },
            note_size: note_size(size),
            p,
        }
    }

    pub fn get(cx: &App) -> &Theme {
        cx.global::<Theme>()
    }
}

/// Size of the writer's text for each setting. Fraunces runs a little smaller on the
/// body than a sans at the same size, so the default is 17.
pub fn note_size(size: TextSize) -> Pixels {
    match size {
        TextSize::Small => px(15.),
        TextSize::Default => px(17.),
        TextSize::Large => px(19.),
    }
}

/// A mind's thread colour. Every one reads at 4.5:1 or better on the page and on a tag,
/// in both lights, because the mind's name on its tag is set in it.
pub fn ink_color(ink: InkColor, p: &Palette) -> Hsla {
    let (light, dark) = thread_hex(ink);
    c(if p.dark { dark } else { light })
}

/// (linen, lamplight) values for each thread.
pub fn thread_hex(ink: InkColor) -> (u32, u32) {
    match ink {
        InkColor::Ink => (0xa63f32, 0xd9705e),    // madder: Inkling's own
        InkColor::Blue => (0x2e3f7a, 0x8c9be0),   // indigo
        InkColor::Green => (0x4f7350, 0x8db88e),  // sage
        InkColor::Red => (0x9b3550, 0xe07a96),    // rose
        InkColor::Purple => (0x6b3e62, 0xc08ab4), // plum
        InkColor::Brown => (0x6e4b2e, 0xc49a73),  // walnut
        InkColor::Orange => (0x8f6214, 0xd9a548), // ochre
    }
}

/// Text colours offered in the selection toolbar: the same threads, for your own words.
/// A note stores the linen value; lamplight draws its lighter twin, so coloured words
/// still read on the dark page.
pub const TEXT_COLORS: [(u32, u32, &str); 8] = [
    (0x000000, 0x000000, "Default"),
    (0x6f6862, 0xa39a8f, "Grey"),
    (0x6e4b2e, 0xc49a73, "Walnut"),
    (0x8f6214, 0xd9a548, "Ochre"),
    (0xa63f32, 0xd9705e, "Madder"),
    (0x4f7350, 0x8db88e, "Sage"),
    (0x2e3f7a, 0x8c9be0, "Indigo"),
    (0x6b3e62, 0xc08ab4, "Plum"),
];

/// How a stored text colour (`Mark::Color`) draws on `p`'s page: 0 is the ink, a
/// toolbar colour is its twin for the light, and any other colour (from an older
/// build) is lifted on the dark page so it does not sink into it.
pub fn text_color(hex: u32, p: &Palette) -> Hsla {
    if hex == 0 {
        return p.ink;
    }
    if let Some((_, dark, _)) = TEXT_COLORS.iter().find(|(light, _, _)| *light == hex) {
        return c(if p.dark { *dark } else { hex });
    }
    let base = c(hex);
    if p.dark && base.l < 0.62 { Hsla { l: (1. - base.l).clamp(0.62, 0.8), ..base } } else { base }
}

pub fn load_fonts(cx: &mut App) {
    let fonts: Vec<Cow<'static, [u8]>> = vec![
        Cow::Borrowed(include_bytes!("../assets/fonts/Figtree-Regular.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/Figtree-Italic.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/Figtree-Medium.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/Figtree-SemiBold.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/Figtree-Bold.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/FrauncesText-Regular.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/FrauncesText-Italic.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/FrauncesText-Medium.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/FrauncesText-SemiBold.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/FrauncesText-SemiBoldItalic.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/FrauncesText-Bold.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/FrauncesText-BoldItalic.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/FrauncesDisplay-Medium.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/FrauncesDisplay-SemiBold.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/SplineSansMono-Regular.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/SplineSansMono-Medium.ttf")),
        Cow::Borrowed(include_bytes!("../assets/fonts/SplineSansMonoInline-Regular.ttf")),
    ];
    if let Err(e) = cx.text_system().add_fonts(fonts) {
        log::error!("could not load bundled fonts: {e:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG contrast ratio of two opaque colours.
    fn contrast(a: Hsla, b: Hsla) -> f32 {
        let lum = |h: Hsla| {
            let c = h.to_rgb();
            let f = |v: f32| if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
            0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.0722 * f(c.b)
        };
        let (x, y) = (lum(a), lum(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[std::prelude::v1::test]
    fn resolves_every_combination() {
        let windows = [WindowAppearance::Light, WindowAppearance::VibrantLight, WindowAppearance::Dark, WindowAppearance::VibrantDark];
        let mut seen = 0;
        for &appearance in Appearance::ALL {
            for window in windows {
                for &ink in InkColor::ALL {
                    for &lettering in Lettering::ALL {
                        for &size in TextSize::ALL {
                            let t = Theme::resolve(appearance, window, ink, lettering, size);
                            let dark = match appearance {
                                Appearance::Light => false,
                                Appearance::Dark => true,
                                Appearance::System => matches!(window, WindowAppearance::Dark | WindowAppearance::VibrantDark),
                            };
                            assert_eq!(t.p.dark, dark, "{appearance:?} {window:?}");
                            assert_eq!(t.p.paper, if dark { Palette::dark().paper } else { Palette::light().paper });
                            assert_eq!(t.voice_ink, ink_color(ink, &t.p));
                            assert_eq!(t.voice_font.as_ref(), if lettering == Lettering::Own { SANS } else { SERIF });
                            assert_eq!(t.note_size, note_size(size));
                            // A mind's name on its tag is set in its thread: 4.5:1 on the page and on tags.
                            for bg in [t.p.paper, t.p.card] {
                                assert!(contrast(t.voice_ink, bg) >= 4.5, "{ink:?} on {appearance:?}/{window:?}: {}", contrast(t.voice_ink, bg));
                            }
                            seen += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(seen, 3 * 4 * 7 * 2 * 3);
    }

    #[std::prelude::v1::test]
    fn palette_follows_the_spec() {
        let light = Palette::light();
        assert_eq!((light.paper, light.ink, light.cloth), (c(0xfbf8f2), c(0x26232b), c(0xefe8db)));
        let dark = Palette::dark();
        assert_eq!((dark.paper, dark.ink, dark.cloth), (c(0x1f1d22), c(0xede6da), c(0x18161a)));
        assert_eq!(ink_color(InkColor::Ink, &light), c(0xa63f32), "madder is Inkling's own thread");
        // quiet text and every label still read on the page, in both lights
        for p in [light, dark] {
            for fg in [p.ink, p.graphite, p.faint] {
                for bg in [p.paper, p.card, p.shelf] {
                    assert!(contrast(fg, bg) >= 4.5, "{fg:?} on {bg:?}: {}", contrast(fg, bg));
                }
            }
        }
        assert_eq!((note_size(TextSize::Small), note_size(TextSize::Default), note_size(TextSize::Large)), (px(15.), px(17.), px(19.)));
    }

    #[std::prelude::v1::test]
    fn coloured_words_read_on_both_pages() {
        for p in [Palette::light(), Palette::dark()] {
            assert_eq!(text_color(0, &p), p.ink, "the default colour is the ink");
            for (light, _, name) in TEXT_COLORS.iter().skip(1) {
                let fg = text_color(*light, &p);
                assert!(contrast(fg, p.paper) >= 4.5, "{name} on the {} page: {}", if p.dark { "dark" } else { "light" }, contrast(fg, p.paper));
            }
            // A colour an older build stored, which the toolbar no longer offers.
            let old = text_color(0x33689c, &p);
            assert!(contrast(old, p.paper) >= 3., "an old colour on the {} page: {}", if p.dark { "dark" } else { "light" }, contrast(old, p.paper));
        }
        assert_eq!(text_color(0x2e3f7a, &Palette::light()), c(0x2e3f7a), "linen draws what was stored");
    }
}

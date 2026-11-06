//! The Dock icon follows the system's light or dark appearance while Inkling is open.
//!
//! macOS swaps an icon by appearance only when it is compiled into an asset catalog,
//! which takes Xcode's `actool`; the bundle ships a plain `.icns`, the light Initial.
//! So while the app runs it sets the Dock's image itself, from the two Initial
//! drawings (`assets/dock-light.png`, `assets/dock-dark.png`), whenever the
//! appearance changes, as every other icon in the Dock does.

use gpui_kit::WindowAppearance;
use std::sync::atomic::{AtomicU8, Ordering};

static LIGHT: &[u8] = include_bytes!("../assets/dock-light.png");
static DARK: &[u8] = include_bytes!("../assets/dock-dark.png");

/// Which image the Dock shows: 0 before the first call, then 1 light, 2 dark. The image
/// is only handed to the Dock when it changes.
static SHOWN: AtomicU8 = AtomicU8::new(0);

/// Whether the system is dark: the Dock's appearance, which is the system's, not
/// the page's (Lamplight on a light system keeps a light Dock).
pub fn is_dark(appearance: WindowAppearance) -> bool {
    matches!(appearance, WindowAppearance::Dark | WindowAppearance::VibrantDark)
}

/// Show the icon that matches `appearance` in the Dock. Call on the main thread.
pub fn follow(appearance: WindowAppearance) {
    let dark = is_dark(appearance);
    let want = if dark { 2 } else { 1 };
    if SHOWN.swap(want, Ordering::Relaxed) == want {
        return;
    }
    set_icon(if dark { DARK } else { LIGHT });
}

#[cfg(target_os = "macos")]
// objc 0.2's macros test `feature = "cargo-clippy"`, which today's compiler flags in the
// crate that expands them; the warning is about the macro, not this code.
#[allow(unexpected_cfgs)]
fn set_icon(png: &'static [u8]) {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};
    // SAFETY: plain AppKit calls on the main thread (GPUI runs the app's callbacks
    // there); the NSData is autoreleased, and the NSImage we own is released once the
    // application has retained it.
    unsafe {
        let data: *mut Object = msg_send![class!(NSData), dataWithBytes: png.as_ptr() as *const std::ffi::c_void length: png.len()];
        if data.is_null() {
            return;
        }
        let image: *mut Object = msg_send![class!(NSImage), alloc];
        let image: *mut Object = msg_send![image, initWithData: data];
        if image.is_null() {
            return;
        }
        let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
        let () = msg_send![app, setApplicationIconImage: image];
        let () = msg_send![image, release];
    }
}

#[cfg(not(target_os = "macos"))]
fn set_icon(_: &'static [u8]) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dock_is_dark_only_when_the_system_is() {
        assert!(is_dark(WindowAppearance::Dark));
        assert!(is_dark(WindowAppearance::VibrantDark));
        assert!(!is_dark(WindowAppearance::Light));
        assert!(!is_dark(WindowAppearance::VibrantLight));
    }

    #[test]
    fn both_icons_are_pngs() {
        for png in [LIGHT, DARK] {
            assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        }
    }
}

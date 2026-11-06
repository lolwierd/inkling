mod app;
mod brain;
mod code;
mod dock;
mod drawing;
mod editor;
mod export;
mod import;
mod loose_ends;
mod margin;
mod note_page;
mod palette;
mod script;
mod settings;
mod sewn;
mod shortcuts;
mod stand;
mod stitch;
mod store;
mod theme;
mod toast;
mod ui;
mod welcome;
mod working;

use gpui_kit::*;

use crate::{app::AppView, brain::Brain, store::Store};

impl Global for Store {}

/// Logs go to stderr. INKLING_LOG_JSON=1 writes one JSON object per line instead
/// (`{"ms":…,"level":…,"target":…,"msg":…}`, `ms` since launch) for tools that read
/// the log, such as `scripts/dev`; script dumps arrive as `msg: "script dump <label>: {…}"`.
fn init_logging() {
    let mut logger = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("inkling=info,warn"));
    if std::env::var("INKLING_LOG_JSON").is_ok_and(|v| !v.is_empty() && v != "0") {
        let start = std::time::Instant::now();
        logger.format(move |buf, record| {
            use std::io::Write as _;
            let line = serde_json::json!({
                "ms": start.elapsed().as_millis() as u64,
                "level": record.level().as_str().to_ascii_lowercase(),
                "target": record.target(),
                "msg": record.args().to_string(),
            });
            writeln!(buf, "{line}")
        });
    }
    logger.init();
}

fn main() {
    init_logging();

    // A script types into notes and starts new ones. Against the real notebook it would
    // write into the owner's notes, so a script only runs with an explicit notebook folder.
    if std::env::var_os("INKLING_SCRIPT").is_some() && std::env::var_os("INKLING_DATA_DIR").is_none_or(|d| d.is_empty()) {
        eprintln!(
            "INKLING_SCRIPT needs INKLING_DATA_DIR: a script edits notes, so it never runs against the real notebook. \
             Use scripts/dev (it makes a scratch notebook), or point INKLING_DATA_DIR at a scratch folder."
        );
        std::process::exit(2);
    }

    let store = match Store::open() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Inkling could not open its data folder: {e:#}");
            std::process::exit(1);
        }
    };

    gpui_kit::application().with_assets(gpui_kit::assets::AllAssets).run(move |cx| {
        gpui_kit::init(cx);
        // One window: closing it ends the app (the page saves on the way out),
        // rather than leaving it running with no window and no way to open one.
        cx.set_quit_mode(QuitMode::LastWindowClosed);
        theme::load_fonts(cx);
        cx.set_global(store);
        app::init(cx);
        Brain::init(cx);

        let bounds = Bounds::centered(None, size(px(1320.), px(860.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                // INKLING_WINDOW_TITLE tells parallel test runs' windows apart in screenshots.
                title: Some(std::env::var("INKLING_WINDOW_TITLE").unwrap_or_else(|_| "Inkling".into()).into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(14.), px(15.))),
            }),
            window_min_size: Some(size(px(720.), px(480.))),
            focus: std::env::var("INKLING_BACKGROUND").is_err(),
            ..Default::default()
        };
        let (window, _view) = gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| AppView::new(window, cx)))
            .expect("failed to open the Inkling window");
        // Test runs pass INKLING_BACKGROUND=1 so launching never takes focus.
        if std::env::var("INKLING_BACKGROUND").is_err() {
            let _ = window.update(cx, |_, window, _| window.activate_window());
            cx.activate(true);
        }
    });
}

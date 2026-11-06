mod app;
mod brain;
mod drawing;
mod editor;
mod export;
mod margin;
mod note_page;
mod palette;
mod script;
mod settings;
mod stitch;
mod store;
mod theme;
mod ui;
mod welcome;

use gpui_kit::*;

use crate::{app::AppView, brain::Brain, store::Store};

impl Global for Store {}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("inkling=info,warn")).init();

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

//! `INKLING_SCRIPT=<file>` plays a short script against the UI at launch: type a
//! paragraph, select words, ask, sketch, chat. It drives the same code paths a
//! person does, which makes it a cheap end-to-end check and a way to demo the app.
//!
//! One step per line, `#` for comments:
//!
//! ```text
//! note: Rate limiter design
//! type: We'll keep one Redis counter per user and reset it every minute.
//! wait: 9000
//! select: reset it every minute
//! sketch
//! ask: What breaks if the reset runs late?
//! activate: 1
//! reply: Fair, what's the cheapest fix?
//! chat: What have we figured out so far?
//! open: settings          # or sidebar, palette, loose (Loose ends), minds, model, export, jmw, welcome
//! ```
//!
//! `wait: ms` waits on the clock. `settle` waits until the page has nothing scheduled
//! or in flight (no pause pending, no read, no thread or chat request, not typing, no
//! `import:` still reading its vault, no thread being gone to from Loose ends), up
//! to `settle: ms` (default 20000); prefer it after typing or asking, so a run does not
//! depend on how busy the machine is.
//!
//! Steps that type or move like a person, for the reading loop:
//!
//! ```text
//! keys: First part.\nSecond part   # type at the caret; \n presses Enter, \s is a space
//! select: \sspaced words\s        # \s works in select too (whitespace around a selection)
//! enter                            # press Enter at the caret
//! caret: end                       # or `start`, or a phrase (lands after it)
//! edit: old words => new words     # replace the first match, as if retyped
//! escape
//! ```
//!
//! Threads are numbered from 1 among the ones not dismissed, in creation order:
//!
//! ```text
//! resolve: 1      reopen: 1      dismiss: 2      mark: 1   (the "?" in Marks only)
//! copy: 1         (logs what Copy would put on the clipboard; the clipboard is left alone)
//! makenote: 1
//! hover: 2        (the pointer rests on thread 2's tag, which unfolds; `hover: 0` moves away)
//! drag: 1 -40 120 (move thread 1's tag by that many pixels, as a finished drag would)
//! ```
//!
//! The note, the reader and the room:
//!
//! ```text
//! mode: ask                 # this note: `ask` waits to be asked, anything else reads along
//! brief: What it is for     # Tell Inkling (empty clears it)
//! mind: Skeptic             # this note's mind, by name
//! room: Inkling, Skeptic    # minds for the next `ask:` (Ask the room, up to three)
//! undo-dismiss              # put back the last dismissed thread
//! jump: next                # or `prev`: the next open thread, as ⌥⌘↓ / ⌥⌘↑
//! open: settings            # sidebar|palette|loose|settings|minds|newmind|model|about|export|jmw
//! close                     # close the sidebar and every overlay
//! quit                      # every script should end with it
//! ```
//!
//! And a few for checking state without a screen:
//!
//! ```text
//! clear                     # Clear in the chat panel
//! switch: Other note title  # open another note, as the sidebar does
//! import: ../fixtures/vault # File > Import from Obsidian… with this folder; a relative
//!                           # path starts at the script's own folder
//! set: pace=slow            # pace=regular|slow, placement=both|right|marks,
//!                           # learn=on|off, mode=thinks|ask (new notes)
//! width: 900                # resize the window to this width
//! size: 900x620             # resize the window to this width and height
//! dump: label               # log the page's state as one JSON line
//! export: md                # log what Export would write (md, html or word: the HTML
//!                           # that becomes the Word file); nothing is saved
//! log: anything             # write a marker into the log
//! shot: label               # a screenshot, for the runner: the app only logs the step
//!                           # and holds still for SHOT_HOLD_MS while `scripts/dev`
//!                           # captures the window into <notebook>/shots/label.png
//! expect: d.threads.length === 1   # skipped here: `scripts/dev e2e` checks it after
//!                                  # the run against the dump above it
//! ```
//!
//! And for driving the window like a person, for screenshots and keyboard checks:
//!
//! ```text
//! key: cmd-k                # a keystroke (several, space separated: `key: cmd-a backspace`)
//! click: 640, 300           # click at a point in the window (logical pixels)
//! point: 640, 300           # move the pointer there (`hover: n` rests it on thread n's tag)
//! size: 900x620             # resize the window to this width and height
//! set: appearance=dark      # also size=small|default|large, ink=madder|indigo|…,
//!                           # lettering=own|notes, pointer=on|off
//! ```
//!
//! A step that cannot do anything (an unknown step, a phrase or thread that is not
//! there) logs a warning starting `script` and the script goes on; `scripts/dev e2e`
//! fails the scenario on such a warning unless the scenario allows it.

/// How long the app holds still after a `shot:` step, so the runner's capture (about
/// half a second) sees the state the shot was asked for.
pub const SHOT_HOLD_MS: u64 = 1000;

#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    Note(String),
    Type(String),
    Wait(u64),
    Select(String),
    Riddle,
    Visualize,
    Ask(String),
    Activate(usize),
    Reply(String),
    Chat(String),
    Mode(String),
    Brief(String),
    Mind(String),
    Open(String),
    Close,
    Quit,
    Keys(String),
    Enter,
    Caret(String),
    Edit(String, String),
    Escape,
    Resolve(usize),
    Reopen(usize),
    Dismiss(usize),
    Mark(usize),
    Copy(usize),
    MakeNote(usize),
    ClearChat,
    Switch(String),
    /// Import a vault, as File > Import from Obsidian… does once a folder is picked.
    Import(String),
    Set(String, String),
    Width(f32),
    Dump(String),
    /// What Export would write in this format (`md`, `html`, `word`), logged.
    Export(String),
    Log(String),
    /// Minds for the next ask (Ask the room): `room: Skeptic, Tutor`.
    Room(Vec<String>),
    /// Put back the last dismissed thread, as Undo does.
    UndoDismiss,
    /// Jump to the next (`jump: next`) or previous (`jump: prev`) open thread.
    Jump(bool),
    /// Rest the pointer on a thread's tag (0 = on none).
    Hover(usize),
    /// Move a thread's tag by (dx, dy) pixels.
    Drag(usize, f32, f32),
    /// A screenshot named by the label. Nothing happens in the app: the runner sees the
    /// step in the log and captures the window (parse adds a `Wait` so it holds still).
    Shot(String),
    /// Wait until the page is settled (`NotePage::settled`), at most this many ms.
    Settle(u64),
    /// Resize the window to (width, height).
    Size(f32, f32),
    /// Keystrokes, dispatched to the window as if typed (`cmd-k`, `alt-cmd-down`).
    Key(Vec<String>),
    /// A left click at a point in the window.
    Click(f32, f32),
    /// The pointer moves to a point in the window.
    Pointer(f32, f32),
}

/// "640, 300" or "640 300" as a point.
fn point_arg(arg: &str) -> Option<(f32, f32)> {
    let mut it = arg.split([',', ' ']).filter(|s| !s.is_empty()).map(|s| s.trim().parse::<f32>());
    match (it.next(), it.next(), it.next()) {
        (Some(Ok(x)), Some(Ok(y)), None) => Some((x, y)),
        _ => None,
    }
}

/// How long `settle` waits at most when the step does not say.
pub const SETTLE_MAX_MS: u64 = 20000;

pub fn parse(src: &str) -> Vec<Step> {
    src.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let (cmd, arg) = match l.split_once(':') {
                Some((c, a)) => (c.trim(), a.trim().to_string()),
                None => (l, String::new()),
            };
            let n = || arg.parse().unwrap_or(1);
            Some(match cmd {
                "note" => Step::Note(arg),
                "type" => Step::Type(arg),
                "wait" => Step::Wait(arg.parse().unwrap_or(1000)),
                "select" => Step::Select(arg.replace("\\s", " ")),
                "probe" | "riddle" => Step::Riddle,
                "sketch" | "visualize" => Step::Visualize,
                "ask" => Step::Ask(arg),
                "activate" => Step::Activate(n()),
                "reply" => Step::Reply(arg),
                "chat" => Step::Chat(arg),
                "mode" => Step::Mode(arg),
                "brief" => Step::Brief(arg),
                "mind" => Step::Mind(arg),
                "open" => Step::Open(arg),
                "close" => Step::Close,
                "quit" => Step::Quit,
                "keys" => Step::Keys(arg.replace("\\n", "\n").replace("\\s", " ")),
                "enter" => Step::Enter,
                "caret" => Step::Caret(arg),
                "edit" => {
                    let Some((old, new)) = arg.split_once("=>") else {
                        log::warn!("script: edit needs `old => new`: {l:?}");
                        return None;
                    };
                    Step::Edit(old.trim().to_string(), new.trim().to_string())
                }
                "escape" => Step::Escape,
                "resolve" => Step::Resolve(n()),
                "reopen" => Step::Reopen(n()),
                "dismiss" => Step::Dismiss(n()),
                "mark" => Step::Mark(n()),
                "copy" => Step::Copy(n()),
                "makenote" => Step::MakeNote(n()),
                "clear" => Step::ClearChat,
                "switch" => Step::Switch(arg),
                "import" => Step::Import(arg),
                "set" => {
                    let Some((k, v)) = arg.split_once('=') else {
                        log::warn!("script: set needs `key=value`: {l:?}");
                        return None;
                    };
                    Step::Set(k.trim().to_lowercase(), v.trim().to_lowercase())
                }
                "width" => match arg.parse() {
                    Ok(w) => Step::Width(w),
                    Err(_) => {
                        log::warn!("script: width needs a number: {l:?}");
                        return None;
                    }
                },
                "dump" => Step::Dump(arg),
                "export" => Step::Export(match arg.to_lowercase().as_str() {
                    "html" => "html".into(),
                    "word" | "docx" => "word".into(),
                    _ => "md".into(),
                }),
                "log" => Step::Log(arg),
                "room" => Step::Room(arg.split(',').map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).collect()),
                "undo-dismiss" => Step::UndoDismiss,
                "jump" => Step::Jump(arg != "prev"),
                "hover" => Step::Hover(arg.parse().unwrap_or(0)),
                "drag" => {
                    let v: Vec<&str> = arg.split_whitespace().collect();
                    match (v.first().and_then(|n| n.parse().ok()), v.get(1).and_then(|x| x.parse().ok()), v.get(2).and_then(|y| y.parse().ok())) {
                        (Some(n), Some(dx), Some(dy)) => Step::Drag(n, dx, dy),
                        _ => {
                            log::warn!("script: drag needs `n dx dy`: {l:?}");
                            return None;
                        }
                    }
                }
                "shot" => Step::Shot(arg),
                "settle" => Step::Settle(arg.parse().unwrap_or(SETTLE_MAX_MS)),
                // for the runner, which checks it after the run
                "expect" => return None,
                "size" => match arg.split_once(['x', 'X']).map(|(w, h)| (w.trim().parse(), h.trim().parse())) {
                    Some((Ok(w), Ok(h))) => Step::Size(w, h),
                    _ => {
                        log::warn!("script: size needs `WIDTHxHEIGHT`: {l:?}");
                        return None;
                    }
                },
                "key" => Step::Key(arg.split_whitespace().map(str::to_string).collect()),
                "click" | "point" => match point_arg(&arg) {
                    Some((x, y)) if cmd == "click" => Step::Click(x, y),
                    Some((x, y)) => Step::Pointer(x, y),
                    None => {
                        log::warn!("script: {cmd} needs `x, y`: {l:?}");
                        return None;
                    }
                },
                _ => {
                    log::warn!("script: unknown step {l:?}");
                    return None;
                }
            })
        })
        .flat_map(|step| match step {
            Step::Shot(_) => vec![step, Step::Wait(SHOT_HOLD_MS)],
            step => vec![step],
        })
        .collect()
}

/// A path a step names, such as `import:`'s folder, for the script now playing.
pub fn path(arg: &str) -> std::path::PathBuf {
    path_from(arg, std::env::var_os("INKLING_SCRIPT").as_deref().map(std::path::Path::new))
}

/// A relative path starts at the script's own folder, so a scenario can name a fixture
/// beside it, wherever it is run from.
fn path_from(arg: &str, script: Option<&std::path::Path>) -> std::path::PathBuf {
    let path = std::path::Path::new(arg);
    match script.and_then(std::path::Path::parent) {
        Some(dir) if path.is_relative() => dir.join(path),
        _ => path.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[std::prelude::v1::test]
    fn parses_steps_and_arguments() {
        let steps = parse(
            "# a comment\n\
             note: Rate limiter: design\n\
             keys: One.\\nTwo\\s\n\
             enter\n\
             edit: a => b => c\n\
             set: Pace = Slow\n\
             width: 900\n\
             dismiss: 2\n\
             resolve\n\
             wait: soon\n\
             bogus: step\n\
             edit: no arrow\n\
             expect: d.threads.length === 1\n\
             shot: after tie\n\
             settle\n\
             settle: 500\n",
        );
        assert_eq!(
            steps,
            vec![
                Step::Note("Rate limiter: design".into()),
                Step::Keys("One.\nTwo ".into()),
                Step::Enter,
                Step::Edit("a".into(), "b => c".into()),
                Step::Set("pace".into(), "slow".into()),
                Step::Width(900.),
                Step::Dismiss(2),
                Step::Resolve(1),
                Step::Wait(1000),
                Step::Shot("after tie".into()),
                Step::Wait(SHOT_HOLD_MS),
                Step::Settle(SETTLE_MAX_MS),
                Step::Settle(500),
            ]
        );
    }

    #[std::prelude::v1::test]
    fn export_names_a_format() {
        assert_eq!(
            parse("export: HTML\nexport: docx\nexport: word\nexport\nexport: md\n"),
            vec![Step::Export("html".into()), Step::Export("word".into()), Step::Export("word".into()), Step::Export("md".into()), Step::Export("md".into())]
        );
    }

    #[std::prelude::v1::test]
    fn import_names_a_folder_beside_the_script() {
        assert_eq!(parse("import: ../fixtures/my vault\n"), vec![Step::Import("../fixtures/my vault".into())]);
        let script = std::path::Path::new("brain/test/e2e/scenarios/u04.txt");
        assert_eq!(path_from("../fixtures/v", Some(script)), std::path::Path::new("brain/test/e2e/scenarios/../fixtures/v"));
        assert_eq!(path_from("/abs/v", Some(script)), std::path::Path::new("/abs/v"));
        assert_eq!(path_from("v", None), std::path::Path::new("v"));
        assert_eq!(path_from("v", Some(std::path::Path::new("bare.txt"))), std::path::Path::new("v"));
    }

    #[std::prelude::v1::test]
    fn parses_window_steps() {
        let steps = parse(
            "size: 900x620\n\
             key: cmd-k  alt-cmd-down\n\
             click: 640, 300\n\
             point: 12 40\n\
             click: 1\n\
             size: wide\n",
        );
        assert_eq!(
            steps,
            vec![
                Step::Size(900., 620.),
                Step::Key(vec!["cmd-k".into(), "alt-cmd-down".into()]),
                Step::Click(640., 300.),
                Step::Pointer(12., 40.),
            ]
        );
    }
}

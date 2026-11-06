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
//! open: settings          # or sidebar, palette, minds, model, export, jmw, welcome
//! ```
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
//! open: settings            # sidebar|palette|settings|minds|newmind|model|about|export|jmw
//! close                     # close the sidebar and every overlay
//! quit                      # every script should end with it
//! ```
//!
//! And a few for checking state without a screen:
//!
//! ```text
//! clear                     # Clear in the chat panel
//! switch: Other note title  # open another note, as the sidebar does
//! set: pace=slow            # pace=regular|slow, placement=both|right|marks,
//!                           # learn=on|off, mode=thinks|ask (new notes)
//! width: 900                # resize the window to this width
//! dump: label               # log the page's state as one JSON line
//! log: anything             # write a marker into the log
//! shot: label               # a screenshot, for the runner: the app only logs the step
//!                           # and holds still for SHOT_HOLD_MS while `scripts/dev`
//!                           # captures the window into <notebook>/shots/label.png
//! expect: d.threads.length === 1   # skipped here: `scripts/dev e2e` checks it after
//!                                  # the run against the dump above it
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
    Set(String, String),
    Width(f32),
    Dump(String),
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
}

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
                // for the runner, which checks it after the run
                "expect" => return None,
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
             shot: after tie\n",
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
            ]
        );
    }
}

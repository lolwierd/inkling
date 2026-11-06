//! Going to a thread from outside the page (Loose ends, ⇧⌘K): its words in view, its tag
//! open, the keyboard in its reply box, as ⌥⌘↓ leaves it. A note opened a moment ago has
//! no layout yet, so the scroll waits for its first paint (`finish_goto`).

use super::*;

/// Words whose top is closer than this to the window's top, or than `GOTO_ROOM` to its
/// bottom, are brought to a third of the way down: room above for the header, and below
/// for the tag that drops open level with them.
const GOTO_TOP: f32 = 100.;
const GOTO_ROOM: f32 = 240.;

/// The scroll offset (0 or less, as the page's handle keeps it) that puts words whose top
/// is at `top` (window y, at scroll `offset`) a third of the way down a window `height`
/// tall; `None` when they are already well in view, so the page does not move.
pub(super) fn goto_offset(top: f32, offset: f32, height: f32) -> Option<f32> {
    if top >= GOTO_TOP && top <= height - GOTO_ROOM {
        return None;
    }
    Some((offset - (top - height / 3.)).min(0.))
}

impl NotePage {
    /// Show thread `id` of the note on the page. Hidden threads are shown again, since you
    /// asked for one; a thread that was dismissed has no tag, so only the caret goes to it.
    pub fn go_to_thread(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(thread) = self.note.threads.iter().find(|t| t.id == id).cloned() else { return };
        if self.just_my_words {
            self.set_hidden(false, cx);
        }
        let start = {
            let ed = self.editor.read(cx);
            ed.anchor(id).map(|a| a.range.start).unwrap_or_else(|| floor_char_boundary(ed.text(), thread.start.min(ed.text().len())))
        };
        // the caret moves without the page following it: `finish_goto` places the words
        self.hold_scroll = true;
        self.editor.update(cx, |e, cx| e.move_cursor_to(start, cx));
        if thread.status == ThreadStatus::Dismissed {
            self.close_thread(cx);
            self.editor.update(cx, |e, cx| e.focus(window, cx));
        } else {
            self.activate(Some(id.to_string()), window, cx);
            if let Some(input) = self.replies.get(id).cloned() {
                input.update(cx, |i, cx| i.focus(window, cx));
            }
        }
        self.goto = Some(id.to_string());
        cx.notify();
    }

    /// From `render`: once the note is laid out, bring the words of the thread gone to into
    /// view. Until then, ask for another frame.
    pub(super) fn finish_goto(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.goto.is_none() {
            return;
        }
        let Some(caret) = self.editor.read(cx).cursor_window_rect() else {
            window.request_animation_frame();
            return;
        };
        self.goto = None;
        self.hold_scroll = false;
        let offset = self.scroll.offset();
        let height = window.viewport_size().height.as_f32();
        if let Some(y) = goto_offset(caret.top().as_f32(), offset.y.as_f32(), height) {
            self.scroll.set_offset(point(offset.x, px(y)));
            cx.notify();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::goto_offset;

    #[std::prelude::v1::test]
    fn words_come_to_a_third_of_the_window_unless_already_in_view() {
        // a 900 px window: words between 100 and 660 stay where they are
        assert_eq!(goto_offset(300., -50., 900.), None);
        assert_eq!(goto_offset(100., 0., 900.), None);
        assert_eq!(goto_offset(660., 0., 900.), None);
        // far below: scrolled down until they sit at 300
        assert_eq!(goto_offset(2300., 0., 900.), Some(-2000.));
        assert_eq!(goto_offset(800., -400., 900.), Some(-900.));
        // above the window: scrolled back up, never past the page's top
        assert_eq!(goto_offset(-500., -1200., 900.), Some(-400.));
        assert_eq!(goto_offset(60., -20., 900.), Some(0.));
    }
}

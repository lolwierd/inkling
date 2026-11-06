//! The window's half of Loose ends (⇧⌘K): the card over the page, ⌘K and Loose ends handing
//! each other what was typed, and going to a thread on any note. The list itself is in
//! `loose_ends.rs`; the page's half in `note_page/goto.rs`.

use gpui_kit::*;

use super::AppView;
use crate::loose_ends::{Card, CardEvent, Key};

impl AppView {
    /// Open Loose ends. Over ⌘K (or over itself) it keeps what was typed there.
    pub(super) fn open_loose_ends(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let query = match (&self.palette, &self.loose) {
            (Some(palette), _) => palette.read(cx).query(cx),
            (None, Some(card)) => card.read(cx).query(cx),
            (None, None) => String::new(),
        };
        self.palette = None;
        let page = self.page.clone();
        let last = self.loose_last.clone();
        let card = cx.new(|cx| Card::new(&page, &query, last.as_ref(), window, cx));
        let sub = cx.subscribe_in(&card, window, |this, _, ev: &CardEvent, window, cx| match ev {
            CardEvent::Go(key, at) => this.go_to_thread(key.clone(), *at, window, cx),
            CardEvent::Dismiss => {
                this.loose = None;
                this.focus_editor(window, cx);
                cx.notify();
            }
        });
        self.loose_sub = Some(sub);
        self.loose = Some(card);
        self.sidebar_open = false;
        cx.notify();
    }

    /// What Loose ends had typed, handed to ⌘K when it opens in its place (`None` when
    /// Loose ends was not open or nothing was typed). Closes Loose ends.
    pub(super) fn take_loose_query(&mut self, cx: &App) -> Option<String> {
        self.loose.take().map(|card| card.read(cx).query(cx)).filter(|q| !q.trim().is_empty())
    }

    /// The card's Return: open the thread's note if it is not the one on the page, then show
    /// the thread there. `at` is the row it was on, so the next ⇧⌘K starts after it.
    fn go_to_thread(&mut self, key: Key, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.loose = None;
        self.loose_last = Some((key.clone(), at));
        if self.page.read(cx).note_id() != key.note {
            self.open_note(key.note.clone(), window, cx);
        }
        // (opening saves the note being left; a note that is gone leaves the page as it was)
        if self.page.read(cx).note_id() == key.note {
            self.page.update(cx, |p, cx| p.go_to_thread(&key.thread, window, cx));
        }
        cx.notify();
    }
}

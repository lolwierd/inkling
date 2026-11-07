//! Reading inside a tag: one bounded conversation, with the reply always within reach.
use super::*;
use gpui_kit::component::text::{TextView, TextViewStyle};

#[derive(Default)]
pub(super) struct Conversation {
    pub scroll: ScrollHandle,
    revision: (usize, usize, bool),
}

impl Conversation {
    pub fn update(&mut self, messages: usize, streaming: usize, active: bool) -> ScrollHandle {
        let revision = (messages, streaming, active);
        if self.revision != revision {
            // Follow new words only while already at the end. Reading older messages
            // must not be interrupted by the next token.
            if !active {
                self.scroll.set_offset(point(px(0.), px(0.)));
            } else if !self.revision.2 || self.scroll.max_offset().y + self.scroll.offset().y <= px(8.) {
                self.scroll.scroll_to_bottom();
            }
            self.revision = revision;
        }
        self.scroll.clone()
    }
}

pub(super) fn markdown(key: String, text: &str, t: &Theme) -> AnyElement {
    let mut table = StyleRefinement::default();
    table.overflow.x = Some(Overflow::Scroll);
    TextView::markdown(SharedString::from(key), text.to_string())
        .selectable(true)
        .scrollable(false)
        .style(TextViewStyle {
            paragraph_gap: rems(0.5),
            heading_font_size: Some(std::sync::Arc::new(|level, _| px(if level <= 2 { 16. } else { 14. }))),
            code_block: StyleRefinement::default().bg(t.p.paper).p(px(8.)).rounded(px(4.)),
            inline_code: HighlightStyle { background_color: Some(t.p.hover), ..Default::default() },
            table,
            is_dark: t.p.dark,
            ..Default::default()
        })
        .into_any_element()
}

impl NotePage {
    /// A new message during an answer always steers: cancel the old request before
    /// starting its replacement, carrying the words already shown into the history.
    pub(super) fn interrupt_thread(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some((request, pending)) = self.pending.remove(id) else { return };
        self.cancel_request(&request, cx);
        if let Pending::Streaming(text) = pending {
            if !text.trim().is_empty() {
                let message = self.partial(id, format!("{}\n\n*Interrupted.*", text.trim_end()));
                if let Some(thread) = self.thread_mut(id) {
                    thread.messages.push(message);
                }
                self.schedule_save(cx);
            }
        }
        self.own_calls.remove(id);
        self.unanswered.remove(id);
    }
}

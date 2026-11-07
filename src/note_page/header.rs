//! The header above a note's title, in two lines, by what each part is for.
//!
//! The first line is quiet and says what the note has been given: the folder it is filed
//! in, the directories its minds may read ("linked to warpbuilds", most often its
//! folder's), and Tell Inkling while it has no brief. These are set once and rarely
//! touched, so they are words in the faint ink, not controls among the others.
//!
//! The second line is the reading. On the left, who reads the note and how, as the
//! sentence it is: "Inkling reads along", the mind and the mode each a press that opens
//! its menu. On the right, what is still open, marked with the gutter's own French knot
//! so the count says what it counts, then Hide threads and Turn over. The mind's dot is
//! its thread colour; the knot is a question. Both lines fade while you write.

use super::*;

/// The mode as the second half of the reader's sentence: "Inkling reads along".
fn mode_words(mode: Mode) -> &'static str {
    match mode {
        Mode::ThinksWithYou => "reads along",
        Mode::WhenYouAsk => "waits to be asked",
    }
}

/// The directories a note's minds may read, in words: the first one's name and how many
/// more. After the folder's name it continues that name ("wb · linked to warpbuilds");
/// on its own it starts the line.
pub(crate) fn linked_label(directories: &[String], after_folder: bool) -> Option<String> {
    let first = directories.first()?;
    let name = std::path::Path::new(first).file_name().and_then(|n| n.to_str()).unwrap_or(first);
    let more = if directories.len() > 1 { format!(" +{}", directories.len() - 1) } else { String::new() };
    Some(format!("{} to {name}{more}", if after_folder { "linked" } else { "Linked" }))
}

fn loose_threads(open: usize) -> String {
    if open == 1 { "1 loose thread".to_string() } else { format!("{open} loose threads") }
}

/// What the header shows, before it is drawn: the render and the dump read the same parts.
struct Parts {
    folder: Option<crate::store::Folder>,
    linked: Option<String>,
    directories: Vec<String>,
    /// The note chose its own directories (or none), over its folder's.
    overridden: bool,
    tell: bool,
    /// The mind's name and the mode's words; none while threads are hidden.
    reader: Option<(String, &'static str)>,
    open: usize,
    hide: Option<&'static str>,
    turn_over: bool,
}

impl Parts {
    fn meta_words(&self) -> Vec<String> {
        let mut w = vec![];
        if let Some(f) = &self.folder { w.push(f.name.clone()) }
        if let Some(l) = &self.linked { w.push(l.clone()) }
        if self.overridden { w.push("Use folder setting".into()) }
        if self.tell { w.push("Tell Inkling".into()) }
        w
    }

    fn row_words(&self) -> Vec<String> {
        let mut w = vec![];
        if let Some((name, how)) = &self.reader { w.push(format!("{name} {how}")) }
        if self.open > 0 { w.push(loose_threads(self.open)) }
        if let Some(h) = self.hide { w.push(h.into()) }
        if self.turn_over { w.push("Turn over".into()) }
        w
    }
}

impl NotePage {
    fn header_parts(&self, cx: &App) -> Parts {
        let store = cx.global::<Store>();
        let directories = self.note.local_directories(&store.folders).to_vec();
        let folder = self.note.folder.as_ref().and_then(|id| store.folders.iter().find(|f| &f.id == id)).cloned();
        let jmw = self.just_my_words;
        Parts {
            linked: linked_label(&directories, folder.is_some()),
            folder,
            directories,
            overridden: self.note.local_directories.is_some(),
            tell: !jmw && self.note.brief.is_none(),
            reader: (!jmw).then(|| (self.mind.name.to_string(), mode_words(self.note.mode))),
            open: if jmw { 0 } else { self.note.threads.iter().filter(|th| th.status == ThreadStatus::Open).count() },
            hide: (jmw || self.note.visible_threads() > 0).then_some(if jmw { "Show threads" } else { "Hide threads" }),
            turn_over: !jmw && !self.note.threads.is_empty(),
        }
    }

    /// The header's words, line by line, for the dump.
    pub(super) fn header_words(&self, cx: &App) -> Value {
        let parts = self.header_parts(cx);
        json!({ "meta": parts.meta_words(), "row": parts.row_words() })
    }

    /// The note's own directories, chosen in the system's panel; they override its
    /// folder's until Use folder setting. File > Link Files to This Note… comes here too,
    /// for a note that is linked to nothing and so has no words to press.
    pub(crate) fn choose_local_directories(&mut self, cx: &mut Context<Self>) {
        let note_id = self.note.id.clone();
        let rx = cx.prompt_for_paths(PathPromptOptions { files: false, directories: true, multiple: true, prompt: Some("Use for this note".into()) });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else { return };
            let _ = this.update(cx, |this, cx| {
                if this.note.id != note_id { return; }
                this.note.local_directories = Some(paths.iter().map(|p| p.to_string_lossy().into_owned()).collect());
                this.flush(cx);
                cx.notify();
            });
        }).detach();
    }

    pub(super) fn header(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let parts = self.header_parts(cx);
        let menu = self.menu;
        let fade = self.chrome.get();

        // The first line: words in the faint ink, lit under the pointer.
        let quiet = move |id: &'static str| {
            div()
                .id(id)
                .flex()
                .items_center()
                .gap(px(5.))
                .h(px(18.))
                .text_color(p.faint)
                .cursor_pointer()
                .hover(move |s| s.text_color(p.ink))
                .active(|s| s.opacity(0.7))
        };
        let mut given: Vec<AnyElement> = vec![];
        if let Some(folder) = parts.folder.clone() {
            given.push(quiet("note-folder")
                .child(icon("folder", 12., p.faint))
                .child(div().max_w(px(240.)).truncate().child(folder.name))
                .keyed_as("header:folder", 4., t.voice_ink, cx, move |_, _, cx| cx.emit(PageEvent::OpenFolder(Some(folder.id.clone()))))
                .into_any_element());
        }
        if let Some(label) = parts.linked.clone() {
            let tip = format!("{}\n{}", if parts.overridden { "Set on this note; overrides its folder" } else { "Inherited from this note's folder" }, parts.directories.join("\n"));
            given.push(quiet("note-files")
                .child(div().max_w(px(220.)).truncate().child(label))
                .tooltip(move |window, cx| gpui_kit::component::tooltip::Tooltip::new(tip.clone()).build(window, cx))
                .keyed_as("header:files", 4., t.voice_ink, cx, |this, _, cx| this.choose_local_directories(cx))
                .into_any_element());
        }
        if parts.overridden {
            given.push(quiet("inherit-files")
                .child("Use folder setting")
                .keyed_as("header:inherit-files", 4., t.voice_ink, cx, |this, _, cx| {
                    this.note.local_directories = None;
                    this.flush(cx);
                    cx.notify();
                })
                .into_any_element());
        }
        if parts.tell {
            given.push(quiet("tell")
                .child(icon("pencil", 11., p.faint))
                .child("Tell Inkling")
                .keyed_as("header:tell", 4., t.voice_ink, cx, |this, window, cx| this.edit_brief(window, cx))
                .into_any_element());
        }
        let given = (!given.is_empty()).then(|| {
            let mut line = div().flex().flex_wrap().items_center().gap_x(px(6.)).gap_y(px(2.)).text_size(px(ts::SMALL)).font_family(SANS).opacity(fade);
            for (i, part) in given.into_iter().enumerate() {
                if i > 0 {
                    line = line.child(div().text_color(p.faint.opacity(0.6)).child("·"));
                }
                line = line.child(part);
            }
            line
        });

        // The second line. Every control is quiet until the pointer is on it, and stays
        // lit while its menu is open.
        let control = move |id: &'static str, lit: bool| {
            div()
                .id(id)
                .flex()
                .items_center()
                .gap(px(5.))
                .h(px(24.))
                .text_size(px(ts::UI))
                .text_color(if lit { p.ink } else { p.graphite })
                .cursor_pointer()
                .hover(move |s| s.text_color(p.ink))
                .active(|s| s.opacity(0.7))
        };
        let mind_menu = (menu == Some(Menu::Mind)).then(|| self.menu_card(Menu::Mind, cx));
        let mode_menu = (menu == Some(Menu::Mode)).then(|| self.menu_card(Menu::Mode, cx));
        // Each menu hangs so its words line up with its part of the sentence: the mind's
        // dot and name over the dot and name; a mode's title under "reads along", its icon
        // out to the left (the card's edge, an item's inset, then a 12 px icon and a gap).
        let (mind_dx, mode_dx) = (-(1. + 5. + 9.), -(1. + 5. + 9. + 12. + 7.));
        let reader = parts.reader.clone().map(|(name, how)| {
            div()
                .flex()
                .items_center()
                // a word space: the mind and its mode read as one sentence
                .gap(px(4.))
                .child(
                    div()
                        .relative()
                        .child(
                            control("mind-picker", menu == Some(Menu::Mind))
                                .gap(px(6.))
                                .text_color(p.ink)
                                .font_weight(FontWeight::MEDIUM)
                                .child(ui::small_dot(t.voice_ink, 7.))
                                // a mind with a long name gives way, so the row still fits the column
                                .child(div().max_w(px(220.)).truncate().child(name))
                                .keyed_as("header:mind", 4., t.voice_ink, cx, |this, window, cx| this.toggle_menu(Menu::Mind, window, cx)),
                        )
                        .children(mind_menu.map(|card| self.drop_down(card, mind_dx, cx))),
                )
                .child(
                    div()
                        .relative()
                        .child(
                            control("mode", menu == Some(Menu::Mode))
                                .child(how)
                                .keyed_as("header:mode", 4., t.voice_ink, cx, |this, window, cx| this.toggle_menu(Menu::Mode, window, cx)),
                        )
                        .children(mode_menu.map(|card| self.drop_down(card, mode_dx, cx))),
                )
        });
        let jmw = self.just_my_words;
        let open = parts.open;
        let row = div()
            .flex()
            .flex_row()
            // a narrow column (a small window with the chat open) wraps the row
            .flex_wrap()
            .items_center()
            .gap_x(px(16.))
            .gap_y(px(2.))
            .min_h(px(24.))
            .opacity(fade)
            .font_family(SANS)
            .children(reader)
            // the open count and the views wrap together, and keep to the right edge on
            // whichever line they land (a spacer that wrapped with them, or did not, put
            // them left or right by turns as the window narrowed)
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(px(16.))
                    .when(open > 0, |d| {
                        d.child(
                            control("open-count", false)
                                .gap(px(6.))
                                // the gutter's knot beside a paragraph with a question
                                .child(ui::knot(t.voice_ink, 8.))
                                .child(loose_threads(open))
                                .keyed_as("header:open-count", 4., t.voice_ink, cx, |this, window, cx| this.jump_to_open(true, window, cx)),
                        )
                    })
                    .when_some(parts.hide, |d, words| {
                        d.child(
                            control("hide-threads", false)
                                .child(words)
                                .keyed_as("header:hide-threads", 4., t.voice_ink, cx, move |this, _, cx| this.set_hidden(!jmw, cx)),
                        )
                    })
                    // the back of the note, once it has had a thread (where Turn back sits on the back)
                    .when(parts.turn_over, |d| {
                        d.child(control("turn-over", false).child("Turn over").keyed_as("header:turn-over", 4., t.voice_ink, cx, |this, window, cx| this.turn_over(window, cx)))
                    }),
            );
        div().flex().flex_col().gap(px(10.)).children(given).child(row).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[std::prelude::v1::test]
    fn linked_words() {
        let one = vec!["/Users/a/Projects/warpbuilds".to_string()];
        let three = vec!["/a/x".to_string(), "/b/y".to_string(), "/c/z".to_string()];
        assert_eq!(linked_label(&[], true), None);
        assert_eq!(linked_label(&one, true).as_deref(), Some("linked to warpbuilds"));
        assert_eq!(linked_label(&one, false).as_deref(), Some("Linked to warpbuilds"));
        assert_eq!(linked_label(&three, true).as_deref(), Some("linked to x +2"));
        // a root has no file name: the path stands in for it
        assert_eq!(linked_label(&["/".to_string()], true).as_deref(), Some("linked to /"));
    }

    #[std::prelude::v1::test]
    fn the_reader_is_one_sentence() {
        assert_eq!(format!("Inkling {}", mode_words(Mode::ThinksWithYou)), "Inkling reads along");
        assert_eq!(format!("Skeptic {}", mode_words(Mode::WhenYouAsk)), "Skeptic waits to be asked");
    }
}

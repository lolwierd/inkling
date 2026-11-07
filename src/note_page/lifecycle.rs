//! Request ownership: cancellation is terminal, deferred work belongs to one visit to
//! a note, and learning is a serial read/modify/write of the owner's profile.
use super::*;
use std::collections::VecDeque;

pub(super) enum Recheck {
    Edit(u64),
    Answer(u64),
}

#[derive(Default)]
pub(super) struct Lifecycle {
    pub epoch: u64,
    // Removing a finished/cancelled request drops its timer and captured payload.
    pub requests: HashMap<String, (Rc<Cell<bool>>, Task<()>)>,
    pub calls: HashMap<String, u64>,
    learning: bool,
    learn_queue: VecDeque<(u64, String)>,
    /// Wakes the page when the Claude plan's hold ends (`wake_after_hold`).
    hold_wake: Option<Task<()>>,
}

impl NotePage {
    pub(super) fn paragraph_in_flight(&self, hash: u64) -> bool {
        self.reading.as_ref().is_some_and(|r| r.hash == hash)
            || self.rechecking.values().any(|r| matches!(r, Recheck::Answer(h) if *h == hash))
            || self.thread_read_in_flight(hash)
    }

    pub(super) fn thread_read_in_flight(&self, hash: u64) -> bool {
        self.rechecking.values().any(|r| matches!(r, Recheck::Edit(h) if *h == hash))
            || self.lifecycle.calls.values().any(|h| *h == hash)
    }

    pub(super) fn quote_is_current(&self, id: &str, words: &str, cx: &App) -> bool {
        let ed = self.editor.read(cx);
        self.note.threads.iter().any(|t| t.id == id && t.status == ThreadStatus::Open)
            && ed.anchor(id).and_then(|a| ed.text().get(a.range.clone())).is_some_and(|s| s == words)
    }

    /// A paused edit that met a busy thread is retried when that request ends. Remove
    /// it before deferring, so two deltas cannot schedule the same recheck.
    /// The Claude plan refused calls: when its hold ends, pick up what waited for it, the
    /// paragraphs left unread, the rechecks and the learning, without waiting for you to
    /// type again. One wake at a time.
    pub(super) fn wake_after_hold(&mut self, cx: &mut Context<Self>) {
        if self.lifecycle.hold_wake.is_some() { return; }
        let Some(left) = crate::brain::hold_left() else { return };
        self.lifecycle.hold_wake = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(left + Duration::from_secs(1)).await;
            let _ = this.update(cx, |page, cx| {
                page.lifecycle.hold_wake = None;
                page.after_read(cx);
                page.resume_rechecks(cx);
                page.next_learning(cx);
            });
        }));
    }

    pub(super) fn resume_rechecks(&mut self, cx: &mut Context<Self>) {
        if self.typing_recently() || crate::brain::holding() { return; }
        let ready: Vec<_> = self.rechecks.keys().filter(|id| !self.pending.contains_key(*id)).cloned().collect();
        for id in ready {
            let Some((_, before)) = self.rechecks.remove(&id) else { continue };
            self.later(cx, move |page, cx| page.recheck(&id, before, cx));
        }
    }

    pub(super) fn leave_requests(&mut self, cx: &mut Context<Self>) {
        self.lifecycle.epoch += 1;
        let ids: Vec<_> = self.lifecycle.requests.keys().cloned().collect();
        for id in ids { self.cancel_request(&id, cx); }
        self.reading = None;
        for (id, _) in std::mem::take(&mut self.pending) { self.drop_if_empty(&id, cx); }
    }

    pub(super) fn maybe_learn(&mut self, message: &str, cx: &mut Context<Self>) {
        if !cx.global::<Store>().settings.learns_about_me { return; }
        let lower = format!(" {} ", message.to_lowercase());
        let personal = [" i ", " i'm ", " im ", " i am ", " my ", " i've ", " i'd ", " me ", " i work", " i study", " i prefer"]
            .iter().any(|k| lower.contains(k));
        if !personal || message.split_whitespace().count() < 4 { return; }
        let revision = cx.global::<Store>().profile_revision();
        self.lifecycle.learn_queue.push_back((revision, message.to_string()));
        self.next_learning(cx);
    }

    fn next_learning(&mut self, cx: &mut Context<Self>) {
        if self.lifecycle.learning { return; }
        // Learning waits, queued, while the Claude plan is refusing calls.
        if crate::brain::holding() {
            self.wake_after_hold(cx);
            return;
        }
        let store = cx.global::<Store>();
        if !store.settings.learns_about_me {
            self.lifecycle.learn_queue.clear();
            return;
        }
        let revision = store.profile_revision();
        self.lifecycle.learn_queue.retain(|(queued, _)| *queued == revision);
        let Some((_, message)) = self.lifecycle.learn_queue.pop_front() else { return };
        let about = &store.settings.about_me;
        let input = json!({"task": "learn", "aboutMe": if about.trim().is_empty() { Value::Null } else { json!(about) }, "message": message});
        self.lifecycle.learning = true;
        self.run_any_note(input, false, cx, move |page, msg, cx| {
            if matches!(msg, BrainMsg::Delta(_)) { return; }
            page.lifecycle.learning = false;
            let store = cx.global_mut::<Store>();
            if let BrainMsg::Done(v) = msg {
                if store.settings.learns_about_me && store.profile_revision() == revision && v["update"].as_bool() == Some(true) {
                    let text = v["aboutMe"].as_str().unwrap_or("").trim();
                    if !text.is_empty() {
                        store.settings.about_me = text.to_string();
                        store.save_settings();
                        // Our own fold is not a manual reset: queued messages still
                        // belong together and must build on the learned profile.
                        let next_revision = store.profile_revision();
                        for (queued, _) in &mut page.lifecycle.learn_queue {
                            if *queued == revision { *queued = next_revision; }
                        }
                        cx.emit(PageEvent::Learned);
                    }
                }
            }
            // Continue even after a stale result: messages submitted after a manual
            // change are still valid. Learning also survives a note switch.
            let this = cx.entity().downgrade();
            cx.defer(move |cx| { if let Some(this) = this.upgrade() { this.update(cx, |page, cx| page.next_learning(cx)); } });
        });
    }
}

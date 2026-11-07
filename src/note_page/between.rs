//! Threads between notes, the page's half of the reading loop: once a paragraph has been
//! read, look for its subject in other notes, and when the model finds that a passage
//! disagrees with it, open one thread that quotes both. The index, the gate and the far ends
//! themselves are in `crate::between`.

use gpui_kit::*;
use serde_json::{Value, json};

use super::{NotePage, Reading};
use crate::between::{self as elsewhere, Passage};
use crate::brain::BrainMsg;
use crate::store::{Elsewhere, Mode, Relation, Side, Store, ThoughtKind, hash_text};

/// How many paragraphs before the one read go with it, for context.
const BEFORE: usize = 2;

impl NotePage {
    /// After the read of `line` (`target` is its words): when the local index finds
    /// passages in other notes that share its subject, ask the model whether one of them
    /// disagrees. Most paragraphs find none, and cost nothing. One look at a time: the
    /// newest paragraph read meanwhile waits for it.
    pub(super) fn look_elsewhere(&mut self, line: usize, target: String, cx: &mut Context<Self>) {
        if self.note.mode != Mode::ThinksWithYou {
            return;
        }
        if self.relating.is_some() {
            self.relate_next = Some((line, target));
            return;
        }
        let found = {
            let store = cx.global::<Store>();
            elsewhere::with_index(|ix| ix.passages(&store.notes, &store.minds, &self.note, &target))
        };
        if found.is_empty() {
            return;
        }
        log::info!(
            "relate: line {line}, {} passage(s) scoring {}",
            found.len(),
            found.iter().map(|p| format!("{:.2}", p.score)).collect::<Vec<_>>().join(", ")
        );
        let input = json!({
            "task": "relate",
            "ctx": self.context(cx),
            "target": target,
            "before": self.paragraphs_before(line, cx),
            "passages": elsewhere::passages_wire(&found),
        });
        let hash = hash_text(&target);
        let req = self.run(input, false, cx, move |page, msg, cx| match msg {
            BrainMsg::Done(v) => {
                page.relating = None;
                page.relate_landed(line, &target, &found, &v, cx);
                page.later(cx, |page, cx| page.relate_waiting(cx));
                cx.notify();
            }
            BrainMsg::Error { kind, message } => {
                page.relating = None;
                if kind != "cancelled" {
                    page.report(&kind, &message, cx);
                }
                page.later(cx, |page, cx| page.relate_waiting(cx));
                cx.notify();
            }
            BrainMsg::Delta(_) => {}
        });
        self.relating = Some(Reading { req, line, hash });
        cx.notify();
    }

    /// The paragraph that waited for the last look.
    fn relate_waiting(&mut self, cx: &mut Context<Self>) {
        if self.relating.is_none() {
            if let Some((line, target)) = self.relate_next.take() {
                self.look_elsewhere(line, target, cx);
            }
        }
    }

    /// Up to two non-empty lines before `line`, nearest last (the title is in the prompt).
    fn paragraphs_before(&self, line: usize, cx: &App) -> Vec<String> {
        let ed = self.editor.read(cx);
        let mut out: Vec<String> = (1..line.min(ed.line_count()))
            .rev()
            .map(|l| ed.line_text(l).trim().to_string())
            .filter(|t| !t.is_empty())
            .take(BEFORE)
            .collect();
        out.reverse();
        out
    }

    /// What `relate` found: a thread on the words here, with its far end in the other note.
    fn relate_landed(&mut self, line: usize, target: &str, found: &[Passage], v: &Value, cx: &mut Context<Self>) {
        if v["speak"].as_bool() != Some(true) {
            return;
        }
        let body = v["text"].as_str().unwrap_or("").trim().to_string();
        let other = v["other"].as_str().unwrap_or("");
        let Some(p) = v["passage"].as_u64().and_then(|i| found.get(i as usize)) else { return };
        if body.is_empty() || other.trim().is_empty() || v["note"].as_str().is_some_and(|n| n != p.note) {
            return;
        }
        let relation = if v["relation"] == "depends" { Relation::Depends } else { Relation::Contradicts };
        let far = {
            let store = cx.global::<Store>();
            store.note(&p.note).and_then(|n| elsewhere::locate_other(n, p.block, other).map(|(r, words)| (r, words, n.title.clone(), n.updated_at)))
        };
        // the other note was deleted while the model looked
        let Some((range, words, title, edited)) = far else { return };
        let quote = v["quote"].as_str().unwrap_or("");
        let Some(id) = self.add_thought(line, target, quote, ThoughtKind::Question, Some(body), cx) else { return };
        self.note.elsewhere.push(Elsewhere {
            thread: id.clone(),
            note: p.note.clone(),
            start: range.start,
            end: range.end,
            quote: words,
            title,
            edited,
            relation,
            side: Side::default(),
            tag_offset: (0., 0.),
            visiting: false,
            placed: false,
        });
        // like any question that lands while you type: folded until you pause
        self.arrived.insert(id, std::time::Instant::now());
        self.schedule_save(cx);
    }

    /// For the script's dump: the look in flight, and every far end on this page.
    pub(super) fn between_state(&self, state: &mut Value, cx: &App) {
        let short = |id: &str| id.chars().take(8).collect::<String>();
        let store = cx.global::<Store>();
        state["relating"] = self.relating.as_ref().map_or(Value::Null, |r| json!({"req": r.req, "line": r.line}));
        state["between"] = Value::Array(
            self.note
                .elsewhere
                .iter()
                .map(|e| {
                    json!({
                        "thread": short(&e.thread),
                        "note": e.note,
                        "title": e.title,
                        "quote": e.quote,
                        "start": e.start,
                        "end": e.end,
                        "relation": e.relation.wire(),
                        "visiting": e.visiting,
                        "placed": e.placed,
                        "gone": store.note(&e.note).is_none(),
                    })
                })
                .collect(),
        );
    }
}

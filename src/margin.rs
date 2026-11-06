//! Margin layout: where each thread card goes so it sits beside its words without
//! overlapping its neighbours.

use gpui_kit::*;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::store::Side;

/// Card heights measured on the previous frame, keyed by thread id.
pub type Heights = Rc<RefCell<HashMap<String, Pixels>>>;

/// Wraps a card and records how tall it laid out, so the next frame can stack
/// cards without overlaps. Asks for one more frame when a height changes.
pub struct Measured {
    id: String,
    child: AnyElement,
    heights: Heights,
    owner: AnyView,
}

pub fn measured(id: String, child: impl IntoElement, heights: Heights, owner: AnyView) -> Measured {
    Measured { id, child: child.into_any_element(), heights, owner }
}

impl IntoElement for Measured {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for Measured {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.prepaint(window, cx);
        let h = bounds.size.height;
        let changed = {
            let mut map = self.heights.borrow_mut();
            let prev = map.insert(self.id.clone(), h);
            prev.is_none_or(|p| (p - h).abs() > px(0.5))
        };
        if changed {
            let owner = self.owner.clone();
            cx.defer(move |cx| cx.notify(owner.entity_id()));
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}

#[derive(Clone, Debug)]
pub struct Slot {
    pub id: String,
    pub side: Side,
    /// Where the anchor's words start, relative to the page body.
    pub anchor_top: Pixels,
    /// Where the card was placed.
    pub top: Pixels,
    pub height: Pixels,
}

/// Stack cards on each side in anchor order; push a card down only as far as it
/// needs to clear the one above. The active card keeps its own spot and the
/// others make room around it.
pub fn place(mut items: Vec<(String, Side, Pixels)>, heights: &HashMap<String, Pixels>, active: Option<&str>, gap: Pixels) -> Vec<Slot> {
    items.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
    let mut out = vec![];
    for side in [Side::Left, Side::Right] {
        let column: Vec<&(String, Side, Pixels)> = items.iter().filter(|i| i.1 == side).collect();
        let h = |id: &str| heights.get(id).copied().unwrap_or(px(60.));
        let mut slots: Vec<Slot> = column
            .iter()
            .map(|(id, side, y)| Slot { id: id.clone(), side: *side, anchor_top: *y, top: *y, height: h(id) })
            .collect();
        let pivot = active.and_then(|a| slots.iter().position(|s| s.id == a));
        match pivot {
            Some(p) => {
                // Below the active card: push down.
                let mut bottom = slots[p].top + slots[p].height + gap;
                for s in slots.iter_mut().skip(p + 1) {
                    if s.top < bottom {
                        s.top = bottom;
                    }
                    bottom = s.top + s.height + gap;
                }
                // Above it: push up, but never above zero.
                let mut ceiling = slots[p].top - gap;
                for s in slots[..p].iter_mut().rev() {
                    if s.top + s.height > ceiling {
                        s.top = (ceiling - s.height).max(px(0.));
                    }
                    ceiling = s.top - gap;
                }
                // If pushing up hit the top, settle downward again.
                let mut bottom = px(f32::MIN);
                for s in slots.iter_mut() {
                    if s.top < bottom {
                        s.top = bottom;
                    }
                    bottom = s.top + s.height + gap;
                }
            }
            None => {
                let mut bottom = px(f32::MIN);
                for s in slots.iter_mut() {
                    if s.top < bottom {
                        s.top = bottom;
                    }
                    bottom = s.top + s.height + gap;
                }
            }
        }
        out.extend(slots);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[std::prelude::v1::test]
    fn stacks_without_overlap() {
        let mut h = HashMap::new();
        h.insert("a".to_string(), px(80.));
        h.insert("b".to_string(), px(80.));
        let slots = place(
            vec![("a".into(), Side::Right, px(10.)), ("b".into(), Side::Right, px(20.))],
            &h,
            None,
            px(12.),
        );
        let a = slots.iter().find(|s| s.id == "a").unwrap();
        let b = slots.iter().find(|s| s.id == "b").unwrap();
        assert_eq!(a.top, px(10.));
        assert!(b.top >= a.top + a.height + px(12.));
    }

    #[std::prelude::v1::test]
    fn active_card_keeps_its_place() {
        let mut h = HashMap::new();
        h.insert("a".to_string(), px(100.));
        h.insert("b".to_string(), px(100.));
        let slots = place(
            vec![("a".into(), Side::Left, px(200.)), ("b".into(), Side::Left, px(240.))],
            &h,
            Some("b"),
            px(10.),
        );
        let a = slots.iter().find(|s| s.id == "a").unwrap();
        let b = slots.iter().find(|s| s.id == "b").unwrap();
        assert_eq!(b.top, px(240.));
        assert!(a.top + a.height <= b.top);
    }

    fn no_overlaps(slots: &[Slot], gap: Pixels) {
        for side in [Side::Left, Side::Right] {
            let mut col: Vec<&Slot> = slots.iter().filter(|s| s.side == side).collect();
            col.sort_by(|a, b| a.top.partial_cmp(&b.top).unwrap());
            for w in col.windows(2) {
                assert!(w[0].top + w[0].height + gap <= w[1].top + px(0.01), "{:?} overlaps {:?}", w[0], w[1]);
            }
            assert!(col.iter().all(|s| s.top >= px(0.)));
        }
    }

    #[std::prelude::v1::test]
    fn active_card_pushes_neighbours_both_ways() {
        let mut h = HashMap::new();
        for id in ["a", "b", "c", "d"] {
            h.insert(id.to_string(), px(50.));
        }
        let items = vec![
            ("a".into(), Side::Right, px(300.)),
            ("b".into(), Side::Right, px(310.)),
            ("c".into(), Side::Right, px(320.)),
            ("d".into(), Side::Right, px(330.)),
        ];
        let slots = place(items, &h, Some("c"), px(10.));
        let top = |id: &str| slots.iter().find(|s| s.id == id).unwrap().top;
        assert_eq!(top("c"), px(320.));
        assert_eq!(top("b"), px(260.));
        assert_eq!(top("a"), px(200.));
        assert_eq!(top("d"), px(380.));
        no_overlaps(&slots, px(10.));
    }

    #[std::prelude::v1::test]
    fn no_room_above_the_active_card_settles_downward() {
        let mut h = HashMap::new();
        h.insert("a".to_string(), px(80.));
        h.insert("b".to_string(), px(80.));
        let slots = place(
            vec![("a".into(), Side::Left, px(10.)), ("b".into(), Side::Left, px(30.))],
            &h,
            Some("b"),
            px(14.),
        );
        let a = slots.iter().find(|s| s.id == "a").unwrap();
        let b = slots.iter().find(|s| s.id == "b").unwrap();
        // The card above cannot go past the top, so the active one gives way.
        assert_eq!(a.top, px(0.));
        assert_eq!(b.top, px(94.));
        no_overlaps(&slots, px(14.));
    }

    #[std::prelude::v1::test]
    fn sides_stack_independently_and_unmeasured_cards_get_a_guess() {
        let h = HashMap::new();
        let slots = place(
            vec![
                ("l1".into(), Side::Left, px(100.)),
                ("r1".into(), Side::Right, px(100.)),
                ("l2".into(), Side::Left, px(110.)),
                ("r2".into(), Side::Right, px(400.)),
            ],
            &h,
            Some("r1"),
            px(12.),
        );
        let get = |id: &str| slots.iter().find(|s| s.id == id).unwrap().clone();
        assert_eq!(get("l1").top, px(100.));
        assert_eq!(get("r1").top, px(100.));
        assert_eq!(get("l2").top, px(100. + 60. + 12.));
        assert_eq!(get("r2").top, px(400.));
        assert_eq!(get("l2").anchor_top, px(110.));
        no_overlaps(&slots, px(12.));
    }
}

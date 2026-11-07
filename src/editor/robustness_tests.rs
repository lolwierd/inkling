    use super::*;
    use std::sync::Arc;
    use std::hint::black_box;

    #[std::prelude::v1::test]
    fn font_size_invalidates_reused_lines() {
        let ts = WindowTextSystem::new(Arc::new(TextSystem::new(Arc::new(NoopTextSystem::new()))));
        let mut st = State::new();
        st.load(vec![(BlockKind::Title, "Title".into()), (BlockKind::Paragraph, "These words should get larger.".into())], vec![], vec![]);
        let snap = |size| DocSnapshot { text: &st.text, kinds: &st.kinds, marks: &st.marks, marked: None, note_size: px(size), palette: Palette::light() };
        let (old, _) = layout_doc(&snap(17.), px(600.), &ts, None);
        let (warm, shaped) = layout_doc(&snap(21.), px(600.), &ts, Some(old));
        let (cold, _) = layout_doc(&snap(21.), px(600.), &ts, None);
        eprintln!("AUDIT font-size: warm={} cold={} shaped={}", warm.lines[1].font_size.as_f32(), cold.lines[1].font_size.as_f32(), shaped);
        assert_eq!(warm.lines[1].font_size, cold.lines[1].font_size);
    }

    #[std::prelude::v1::test]
    fn gutter_code_unit_scaling() {
        for n in [1000, 3000, 10000, 20000] {
            let mut st = State::new();
            let mut lines = vec![(BlockKind::Title, "Code".into())];
            lines.extend((0..n).map(|_| (BlockKind::Code(Lang::new("rust")), "let x = 1;".into())));
            st.load(lines, vec![], vec![]);
            let mut times = Vec::new();
            for _ in 0..5 {
                let begin = Instant::now();
                let mut line = 1;
                while line < st.line_count() { let unit = st.unit_of(black_box(line)); line = unit.end; black_box(unit); }
                times.push(begin.elapsed().as_secs_f64() * 1000.);
            }
            times.sort_by(f64::total_cmp);
            eprintln!("AUDIT gutter-code: lines={} median_ms={:.3}", n, times[2]);
        }
    }

    #[std::prelude::v1::test]
    fn undo_snapshot_memory() {
        let mut st = State::new();
        st.load(vec![(BlockKind::Title, "Big".into()), (BlockKind::Paragraph, "x".repeat(1_000_000))], vec![], vec![]);
        for _ in 0..UNDO_LIMIT { st.edit(4..4, "x", EditKind::Other); }
        let bytes: usize = st.undo.iter().map(|s| s.text.capacity()).sum();
        assert!(bytes <= UNDO_BYTES, "retained {bytes} bytes");
        let written = st.text.clone();
        st.undo();
        assert_ne!(st.text, written);
        st.redo();
        assert_eq!(st.text, written);
        eprintln!("AUDIT undo: limit={} document_bytes={} snapshot_text_capacity={}", UNDO_LIMIT, st.text.len(), bytes);
    }
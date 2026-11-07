    use super::*;
    use crate::store::Block;
    use std::{hint::black_box, time::Instant};
    #[std::prelude::v1::test]
    fn sidebar_counts_scaling() {
        for count in [1000, 3000, 10000] {
            let notes: Vec<Note> = (0..count).map(|i| Note::new(&format!("Note {i}"), crate::store::Mode::WhenYouAsk, None)).collect();
            let mut times = vec![];
            for _ in 0..5 {
                let start = Instant::now();
                black_box(open_counts(black_box(&notes)));
                times.push(start.elapsed().as_secs_f64()*1000.);
            }
            times.sort_by(f64::total_cmp);
            eprintln!("AUDIT sidebar: notes={} median_ms={:.3}", count, times[2]);
        }
    }
    #[std::prelude::v1::test]
    fn linked_save_retries_after_failure() {
        let dir = std::env::temp_dir().join("inkling-agent").join(format!("audit-linked-{}", crate::store::new_id()));
        let mut store = Store::open_at(dir.clone()).unwrap();
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("brain/test/e2e/fixtures/between/notes");
        store.notes = std::fs::read_dir(fixture).unwrap().map(|e| serde_json::from_slice::<Note>(&std::fs::read(e.unwrap().path()).unwrap()).unwrap()).collect();
        for note in &store.notes { store.try_save_note(note).unwrap(); }
        let mut away = store.notes.iter().find(|n| n.title == "Race calendar").unwrap().clone();
        add_visitors(&mut away, &store.notes);
        let t = away.threads.iter_mut().find(|t| t.id == "b1").unwrap();
        t.status = ThreadStatus::Resolved;
        let home_id = store.notes.iter().find(|n| n.title == "Race-week plan").unwrap().id.clone();
        let path = dir.join("notes").join(format!("{home_id}.json"));
        let obstruction = path.with_extension("json.tmp");
        std::fs::create_dir(&obstruction).unwrap();
        save_visitors_home(&away, &mut store);
        std::fs::remove_dir(&obstruction).unwrap();
        save_visitors_home(&away, &mut store);
        let disk: Note = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let memory = store.note(&home_id).unwrap().threads.iter().find(|t| t.id == "b1").unwrap().status;
        let persisted = disk.threads.iter().find(|t| t.id == "b1").unwrap().status;
        eprintln!("AUDIT linked-save: memory={memory:?} disk={persisted:?} dir={}", dir.display());
        assert_eq!(persisted, memory);
    }
    #[std::prelude::v1::test]
    fn aggregate_counts_match_individual_counts_with_visiting_ends() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("brain/test/e2e/fixtures/between/notes");
        let mut notes: Vec<Note> = std::fs::read_dir(fixture).unwrap().map(|e| serde_json::from_slice(&std::fs::read(e.unwrap().path()).unwrap()).unwrap()).collect();
        let source = notes.clone();
        for n in &mut notes { add_visitors(n, &source); }
        let counts = open_counts(&notes);
        for n in &notes { assert_eq!(counts.get(n.id.as_str()).copied().unwrap_or(0), open_here(&notes, n)); }
    }

    #[std::prelude::v1::test]
    fn a_mind_rename_rebuilds_searchable_passages() {
        let mut note = Note::new("Plan", crate::store::Mode::ThinksWithYou, None);
        note.blocks.push(Block { kind: BlockKind::Paragraph, text: "Skeptic: Check the marathon training schedule before the race.".into() });
        let mut mind = Mind::inkling(); mind.name = "Other".into();
        let mut index = Index::default();
        index.refresh(&[note.clone()], &[mind.clone()]);
        let before = index.units;
        assert_eq!(before, 1);
        mind.name = "Skeptic".into();
        index.refresh(&[note], &[mind]);
        assert_eq!(index.units, 0, "the renamed mind's own words must be excluded");
    }

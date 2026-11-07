    use super::*;
    #[std::prelude::v1::test]
    fn import_must_not_report_failed_save_as_imported() {
        let dir = std::env::temp_dir().join("inkling-agent").join(format!("audit-import-{}", new_id()));
        let mut store = Store::open_at(dir.clone()).unwrap();
        fs::rename(dir.join("notes"), dir.join("saved-notes")).unwrap();
        fs::write(dir.join("notes"), "obstruct directory").unwrap();
        let vault = Vault { notes: vec![Imported { folders: vec![], blocks: convert("# Lost import\n\nWords that must be saved.", "lost.md", false), created: None, modified: None }], ..Vault::default() };
        let summary = add_to_store(&mut store, vault, Mode::WhenYouAsk);
        eprintln!("AUDIT import: reported={} memory={} disk_note_exists={} dir={}", summary.imported, store.notes.len(), dir.join("notes").is_dir(), dir.display());
        assert_eq!(summary.imported, 0, "failed saves cannot be counted as imported");
        assert_eq!(summary.unsaved, 1);
        assert!(!store.notes.iter().any(|n| n.title == "Lost import"));
        assert!(summary.message("Vault").contains("could not be saved"));
    }
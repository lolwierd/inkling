use super::*;

fn store() -> Store {
    Store::open_at(std::env::temp_dir().join("inkling-agent").join(format!("store-regression-{}", new_id()))).unwrap()
}

#[std::prelude::v1::test]
fn failed_saves_retry_the_newest_value() {
    let mut store = store();
    let obstruction = store.dir.join("settings.json.tmp");
    fs::create_dir(&obstruction).unwrap();
    store.settings.about_me = "old".into(); store.save_settings();
    store.settings.about_me = "new".into(); store.save_settings();
    assert_eq!(store.failed.borrow().len(), 1);
    assert_eq!(store.take_save_problems().len(), 1);
    fs::remove_dir(obstruction).unwrap();
    store.retry_failed();
    let disk: Settings = serde_json::from_slice(&fs::read(store.dir.join("settings.json")).unwrap()).unwrap();
    assert_eq!(disk.about_me, "new");
    assert!(store.failed.borrow().is_empty());
}

#[std::prelude::v1::test]
fn an_explicit_successful_save_supersedes_a_failed_older_edit() {
    let store = store();
    let mut note = store.notes[0].clone();
    let path = store.dir.join("notes").join(format!("{}.json", note.id));
    let obstruction = path.with_extension("json.tmp"); fs::create_dir(&obstruction).unwrap();
    note.title = "Before the move".into(); store.save_note(&note);
    fs::remove_dir(obstruction).unwrap();
    note.title = "After the move".into(); store.try_save_note(&note).unwrap();
    store.retry_failed();
    let disk: Note = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(disk.title, "After the move");
    assert!(store.failed.borrow().is_empty());
    assert!(store.take_save_problems().is_empty());
}

#[std::prelude::v1::test]
fn failed_deletion_keeps_the_note_and_success_cannot_resurrect_it() {
    let mut store = store();
    let note = store.notes[0].clone();
    let path = store.dir.join("notes").join(format!("{}.json", note.id));
    let backup = path.with_extension("backup");
    fs::rename(&path, &backup).unwrap(); fs::create_dir(&path).unwrap();
    assert!(store.delete_note(&note.id).is_err());
    assert!(store.note(&note.id).is_some());
    fs::remove_dir(&path).unwrap(); fs::rename(backup, &path).unwrap();
    let obstruction = path.with_extension("json.tmp"); fs::create_dir(&obstruction).unwrap();
    store.save_note(&note);
    assert_eq!(store.failed.borrow().len(), 1);
    store.delete_note(&note.id).unwrap();
    fs::remove_dir(obstruction).unwrap(); store.retry_failed();
    assert!(!path.exists()); assert!(store.note(&note.id).is_none());
    assert!(store.take_save_problems().is_empty());
}

#[std::prelude::v1::test]
fn recovered_saves_only_report_files_that_are_still_unsaved() {
    let mut store = store();
    let settings = store.dir.join("settings.json.tmp");
    let minds = store.dir.join("minds.json.tmp");
    fs::create_dir(&settings).unwrap(); fs::create_dir(&minds).unwrap();
    store.settings.about_me = "Retry me".into(); store.save_settings(); store.save_minds();
    fs::remove_dir(settings).unwrap(); store.retry_failed();
    let problems = store.take_save_problems();
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("minds.json"));
    fs::remove_dir(minds).unwrap(); store.retry_failed();
    assert!(store.take_save_problems().is_empty());
    assert!(store.failed.borrow().is_empty());
}

#[std::prelude::v1::test]
fn a_new_successful_edit_clears_an_unreported_save_problem() {
    let store = store();
    let mut note = store.notes[0].clone();
    let path = store.dir.join("notes").join(format!("{}.json", note.id));
    let obstruction = path.with_extension("json.tmp");
    fs::create_dir(&obstruction).unwrap(); store.save_note(&note);
    fs::remove_dir(obstruction).unwrap();
    note.title = "Now saved".into(); store.save_note(&note);
    assert!(store.take_save_problems().is_empty());
    let disk: Note = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(disk.title, "Now saved");
}

#[std::prelude::v1::test]
fn manual_profile_edits_and_off_on_cycles_invalidate_learning() {
    let mut store = store();
    let start = store.profile_revision();
    store.settings.learns_about_me = false; store.save_settings();
    store.settings.learns_about_me = true; store.save_settings();
    assert_ne!(store.profile_revision(), start);
    let start = store.profile_revision();
    store.settings.about_me = "Manual edit".into(); store.save_settings();
    store.settings.about_me.clear(); store.save_settings();
    assert_ne!(store.profile_revision(), start);
}

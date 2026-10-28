use keel_core::commands::{Action, Command};
use keel_service::diag::{format_line, DiagLog};
use keel_service::AppService;

#[test]
fn diag_setting_defaults_off_and_persists() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("keel.db");
    let path = path.to_str().unwrap();
    let svc = AppService::open(path).unwrap();
    assert!(!svc.diag_enabled().unwrap());
    svc.set_diag_enabled(true).unwrap();
    drop(svc);
    let svc = AppService::open(path).unwrap();
    assert!(svc.diag_enabled().unwrap());
    svc.set_diag_enabled(false).unwrap();
    assert!(!svc.diag_enabled().unwrap());
}

#[test]
fn format_line_has_time_source_and_one_line() {
    assert_eq!(format_line(1_700_000_000_123, "js", "hello"), "1700000000.123 [js] hello\n");
    assert_eq!(format_line(5_007, "rust", "a\nb"), "5.007 [rust] a\\nb\n");
}

#[test]
fn append_writes_lines() {
    let dir = tempfile::tempdir().unwrap();
    let log = DiagLog::new(dir.path());
    log.append("js", "one").unwrap();
    log.append("rust", "two").unwrap();
    let text = std::fs::read_to_string(dir.path().join("diag.log")).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].ends_with("[js] one"));
    assert!(lines[1].ends_with("[rust] two"));
}

#[test]
fn rolls_over_at_the_cap_keeping_one_previous_file() {
    let dir = tempfile::tempdir().unwrap();
    let log = DiagLog::with_cap(dir.path(), 200);
    for i in 0..40 {
        log.append("js", &format!("line number {i}")).unwrap();
    }
    let cur = dir.path().join("diag.log");
    let prev = dir.path().join("diag.1.log");
    assert!(prev.exists());
    assert!(!dir.path().join("diag.2.log").exists());
    assert!(std::fs::metadata(&cur).unwrap().len() <= 200 + 60);
    // The newest line survives in the current file.
    assert!(std::fs::read_to_string(&cur).unwrap().contains("line number 39"));
}

#[test]
fn start_fresh_keeps_the_last_run_as_the_previous_file() {
    let dir = tempfile::tempdir().unwrap();
    let log = DiagLog::new(dir.path());
    log.append("js", "old run").unwrap();
    log.start_fresh().unwrap();
    log.append("js", "new run").unwrap();
    let cur = std::fs::read_to_string(dir.path().join("diag.log")).unwrap();
    let prev = std::fs::read_to_string(dir.path().join("diag.1.log")).unwrap();
    assert!(cur.contains("new run") && !cur.contains("old run"));
    assert!(prev.contains("old run"));
    // Nothing to keep yet: still fine.
    let empty = tempfile::tempdir().unwrap();
    DiagLog::new(empty.path()).start_fresh().unwrap();
}

/// On desktop the log sits next to keel.db: logging, rolling over, starting
/// fresh and switching the setting must keep the database (file and tasks)
/// and leave every other file alone.
#[test]
fn logging_never_touches_the_database_or_other_files() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("keel.db");
    let svc = AppService::open(db.to_str().unwrap()).unwrap();
    let inbox = svc.list_projects().unwrap().into_iter().find(|p| p.is_inbox).unwrap();
    svc.dispatch(Command {
        uuid: "u1".to_string(),
        temp_id: Some("tmp-1".to_string()),
        action: Action::TaskAdd {
            content: "keep me".to_string(),
            project_id: inbox.id.clone(),
            description: None,
            priority: None,
            due: None,
            labels: None,
            parent_id: None,
            section_id: None,
            recurrence: None,
            deadline: None,
        },
    })
    .unwrap();
    svc.set_diag_enabled(true).unwrap();
    let others = ["notes.txt", "backup.db", "diag.2.log"];
    for f in others {
        std::fs::write(dir.path().join(f), f).unwrap();
    }
    let log = DiagLog::with_cap(dir.path(), 100);
    for i in 0..30 {
        log.append("js", &format!("line {i}")).unwrap();
    }
    log.start_fresh().unwrap();
    log.start_fresh().unwrap();
    svc.set_diag_enabled(false).unwrap();
    svc.set_diag_enabled(true).unwrap();
    svc.set_diag_enabled(false).unwrap();
    drop(svc);
    // Checked before reopening: opening would quietly create a new database.
    assert!(db.exists(), "keel.db was deleted");
    for f in others {
        assert_eq!(std::fs::read_to_string(dir.path().join(f)).unwrap(), f);
    }
    let svc = AppService::open(db.to_str().unwrap()).unwrap();
    assert!(!svc.diag_enabled().unwrap());
    let tasks = svc.list_tasks(inbox.id.as_str()).unwrap();
    assert_eq!(tasks.iter().map(|t| t.content.as_str()).collect::<Vec<_>>(), ["keep me"]);
    let mut names: Vec<_> = std::fs::read_dir(dir.path()).unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
    names.sort();
    assert!(names.iter().all(|n| n.starts_with("keel.db") || others.contains(&n.as_str()) || n == "diag.log" || n == "diag.1.log"), "{names:?}");
}

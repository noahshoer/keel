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

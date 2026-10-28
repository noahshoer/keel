//! The diagnostics log file written by Rust (desktop; on iOS the Swift
//! plugin owns `Library/Caches/diag.log` and rolls it the same way).

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// The file rolls over to `diag.1.log` once it reaches this size.
pub const DEFAULT_CAP: u64 = 2 * 1024 * 1024;

/// One log line: "seconds.millis [source] message". Newlines inside the
/// message are escaped so every entry stays on one line.
pub fn format_line(ts_ms: i64, source: &str, msg: &str) -> String {
    let msg = msg.replace('\r', "").replace('\n', "\\n");
    format!("{}.{:03} [{source}] {msg}\n", ts_ms.div_euclid(1000), ts_ms.rem_euclid(1000))
}

pub struct DiagLog {
    path: PathBuf,
    previous: PathBuf,
    cap: u64,
}

impl DiagLog {
    pub fn new(dir: &Path) -> Self {
        Self::with_cap(dir, DEFAULT_CAP)
    }

    pub fn with_cap(dir: &Path, cap: u64) -> Self {
        DiagLog { path: dir.join("diag.log"), previous: dir.join("diag.1.log"), cap }
    }

    pub fn append(&self, source: &str, msg: &str) -> io::Result<()> {
        let line = format_line(chrono::Utc::now().timestamp_millis(), source, msg);
        if fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0) >= self.cap {
            let _ = fs::remove_file(&self.previous);
            fs::rename(&self.path, &self.previous)?;
        }
        let mut f = OpenOptions::new().create(true).append(true).open(&self.path)?;
        f.write_all(line.as_bytes())
    }
}

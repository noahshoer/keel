// The one Swift command, invoked from Rust (`tauri_plugin_keel_ios::diag`).
const COMMANDS: &[&str] = &["diag"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).ios_path("ios").build();
}

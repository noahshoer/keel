fn main() {
    // The glass capability exists only when its plugin is built in.
    let capabilities = if std::env::var_os("CARGO_FEATURE_GLASS").is_some() {
        "./capabilities/**/*"
    } else {
        "./capabilities/base/*"
    };
    tauri_build::try_build(tauri_build::Attributes::new().capabilities_path_pattern(capabilities))
        .expect("tauri build");
}

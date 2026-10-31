//! The IPC command list and each command's argument type, for the web app.
//!
//! `exports_command_names` reads the `generate_handler![…]` list in
//! `src/lib.rs` and writes the invoke names (bare function names, so
//! `diag::diag_log` is `diag_log`) sorted to `web/src/bindings/commands.json`.
//! `exports_args_bindings` (feature `ts`) writes every command's `…Args`
//! TypeScript type next to the keel-core bindings. `scripts/check-bindings.sh`
//! runs both and fails when the committed files are stale.

use std::path::PathBuf;

/// `TS_RS_EXPORT_DIR` from `.cargo/config.toml` (absolute: `relative = true`).
fn bindings_dir() -> PathBuf {
    PathBuf::from(env!("TS_RS_EXPORT_DIR"))
}

/// The invoke names registered in `generate_handler![…]`, in source order.
fn command_names() -> Vec<String> {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs")).expect("read lib.rs");
    let start = src.find("generate_handler![").expect("generate_handler! in lib.rs") + "generate_handler![".len();
    let end = start + src[start..].find(']').expect("closing ]");
    src[start..end]
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.rsplit("::").next().unwrap_or(s).to_string())
        .collect()
}

#[test]
fn exports_command_names() {
    let mut names = command_names();
    names.sort();
    let mut json = serde_json::to_string_pretty(&names).expect("serialize names");
    json.push('\n');
    std::fs::write(bindings_dir().join("commands.json"), json).expect("write commands.json");

    assert_eq!(names.len(), 30, "{names:?}");
    for n in ["today_view", "dispatch", "diag_log", "set_reminder_settings"] {
        assert!(names.iter().any(|x| x == n), "{n} missing: {names:?}");
    }
    assert!(!names.iter().any(|x| x.contains("::")), "{names:?}");
}

/// Liquid Glass is parked: no config turns the `glass` feature on, so the
/// app never loads the plugin, and Tauri serves the React app (`web/dist`).
#[test]
fn glass_is_opt_in() {
    let dir = env!("CARGO_MANIFEST_DIR");
    let cargo = std::fs::read_to_string(format!("{dir}/Cargo.toml")).expect("read Cargo.toml");
    let dep = cargo.lines().find(|l| l.starts_with("tauri-plugin-glass")).expect("glass dependency");
    assert!(dep.contains("optional = true"), "{dep}");
    let default = cargo.lines().find(|l| l.starts_with("default =")).unwrap_or("default = []");
    assert!(!default.contains("glass"), "{default}");

    let conf: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(format!("{dir}/tauri.conf.json")).expect("tauri.conf.json"))
            .expect("tauri.conf.json");
    let features = &conf["build"]["features"];
    assert!(features.is_null() || features == &serde_json::json!([]), "{features}");
    assert_eq!(conf["build"]["frontendDist"], "../../web/dist");
}

/// The capabilities a build without glass loads never name the glass plugin.
#[test]
fn base_capabilities_skip_glass() {
    let base = concat!(env!("CARGO_MANIFEST_DIR"), "/capabilities/base");
    let files: Vec<_> = std::fs::read_dir(base).expect("capabilities/base").map(|e| e.expect("entry").path()).collect();
    assert!(!files.is_empty());
    for f in files {
        let text = std::fs::read_to_string(&f).expect("read capability");
        assert!(!text.contains("glass:"), "{}", f.display());
        assert!(text.contains("keel-ios:default"), "{}", f.display());
    }
}

#[cfg(feature = "ts")]
mod ts {
    use super::{bindings_dir, command_names};
    use keel_core::commands::ApplyResult;
    use keel_core::filter::FilterGroup;
    use keel_core::import::todoist::ImportSummary;
    use keel_core::model::{Filter, Label, Project, Reminder, Section, Task};
    use keel_core::reminders::{PlannedReminder, ReminderSettings};
    use keel_core::search::SearchResults;
    use keel_lib::args::*;
    use ts_rs::TS;

    /// Every command with the type its `args` parameter takes and the type
    /// it resolves to (the `T` of `Result<T, String>`; `()` is `null`). A
    /// command missing here (or listed but not registered) fails the test.
    /// Returns the names and one `Commands.ts` member per command.
    macro_rules! command_args {
        ($($cmd:ident => $t:ty => $r:ty),* $(,)?) => {{
            $( <$t as TS>::export_all().expect(concat!("export ", stringify!($t))); )*
            vec![$( (stringify!($cmd).to_string(), <$t as TS>::name(), <$r as TS>::name()) ),*]
        }};
    }

    /// The type names a TypeScript type expression refers to (`Array<Task>`
    /// → `Task`); built-ins are lowercase or `Array`.
    fn named_types(ts: &str) -> Vec<String> {
        ts.split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| w.starts_with(|c: char| c.is_ascii_uppercase()) && *w != "Array")
            .map(str::to_string)
            .collect()
    }

    /// `Commands.ts`: each command's args and return type, imported from
    /// the bindings next to it.
    fn write_commands(dir: &std::path::Path, commands: &[(String, String, String)]) {
        let mut imports: Vec<String> =
            commands.iter().flat_map(|(_, a, r)| named_types(a).into_iter().chain(named_types(r))).collect();
        imports.sort();
        imports.dedup();
        let mut out = String::from(
            "// Generated by crates/src-tauri/tests/commands_list.rs. Do not edit.\n\
             // Every Tauri command: its `args` type and what it resolves to.\n",
        );
        for i in &imports {
            assert!(dir.join(format!("{i}.ts")).exists(), "{i}.ts not generated");
            out.push_str(&format!("import type {{ {i} }} from \"./{i}\";\n"));
        }
        out.push_str("\nexport interface Commands {\n");
        for (cmd, args, ret) in commands {
            out.push_str(&format!("  {cmd}: {{ args: {args}; ret: {ret} }};\n"));
        }
        out.push_str("}\n");
        std::fs::write(dir.join("Commands.ts"), out).expect("write Commands.ts");
    }

    /// Same as `crates/core/tests/ts_bindings.rs`, so either test leaves a
    /// complete `index.ts`.
    fn write_index(dir: &std::path::Path) {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .expect("read bindings dir")
            .map(|e| e.expect("dir entry").file_name().to_string_lossy().into_owned())
            .filter_map(|n| n.strip_suffix(".ts").map(str::to_string))
            .filter(|n| n != "index")
            .collect();
        // Case-insensitive, as Biome orders exports.
        names.sort_by_key(|n| n.to_lowercase());
        let mut out = String::from("// Generated by crates/core/tests/ts_bindings.rs. Do not edit.\n");
        for n in names {
            out.push_str(&format!("export type * from \"./{n}\";\n"));
        }
        std::fs::write(dir.join("index.ts"), out).expect("write index.ts");
    }

    #[test]
    fn exports_args_bindings() {
        let commands = command_args!(
            list_projects => NoArgs => Vec<Project>,
            list_tasks => ListTasksArgs => Vec<Task>,
            today_view => NoArgs => Vec<Task>,
            all_tasks => NoArgs => Vec<Task>,
            upcoming_view => UpcomingViewArgs => Vec<Task>,
            project_view => ProjectViewArgs => Vec<Task>,
            list_labels => NoArgs => Vec<Label>,
            label_view => LabelViewArgs => Vec<Task>,
            completed_today => NoArgs => Vec<Task>,
            completed_in_project => CompletedInProjectArgs => Vec<Task>,
            completed_with_label => CompletedWithLabelArgs => Vec<Task>,
            list_sections => ListSectionsArgs => Vec<Section>,
            list_filters => NoArgs => Vec<Filter>,
            check_filter => CheckFilterArgs => (),
            filter_view => FilterViewArgs => Vec<FilterGroup>,
            dispatch => DispatchArgs => ApplyResult,
            search => SearchArgs => SearchResults,
            list_reminders => NoArgs => Vec<Reminder>,
            import_todoist_csv => ImportTodoistCsvArgs => ImportSummary,
            get_reminder_settings => NoArgs => ReminderSettings,
            set_reminder_settings => SetReminderSettingsArgs => (),
            upcoming_reminders => NoArgs => Vec<PlannedReminder>,
            notification_permission => NoArgs => String,
            reminders_when_closed => NoArgs => bool,
            diag_enabled => NoArgs => bool,
            set_diag_enabled => SetDiagEnabledArgs => (),
            diag_log => DiagLogArgs => (),
            request_notification_permission => NoArgs => String,
            test_notification => NoArgs => (),
            hold_page_scroll => HoldPageScrollArgs => (),
        );
        let mut covered: Vec<String> = commands.iter().map(|(c, _, _)| c.clone()).collect();
        covered.sort();
        let mut names = command_names();
        names.sort();
        assert_eq!(covered, names);

        let dir = bindings_dir();
        write_commands(&dir, &commands);
        write_index(&dir);

        let no_args = std::fs::read_to_string(dir.join("NoArgs.ts")).expect("NoArgs.ts written");
        assert!(no_args.contains("export type NoArgs ="), "{no_args}");
        let upcoming = std::fs::read_to_string(dir.join("UpcomingViewArgs.ts")).expect("UpcomingViewArgs.ts");
        assert!(upcoming.contains("days: number"), "{upcoming}");
        let dispatch = std::fs::read_to_string(dir.join("DispatchArgs.ts")).expect("DispatchArgs.ts");
        assert!(dispatch.contains("cmd: Command"), "{dispatch}");
    }
}

# Keel's patched copy of tauri-plugin-notification 2.7.0

Vendored from crates.io (MIT / Apache-2.0, see the license files) and used
through `[patch.crates-io]` in the workspace `Cargo.toml`.

One change, in `ios/Sources/NotificationHandler.swift` (`toActiveNotification`,
marked `KEEL PATCH`): the original force-unwrapped an in-memory map that only
holds notifications scheduled by the current run of the app. Tapping a
notification scheduled by an earlier run (e.g. the daily summary) cold-launches
the app with an empty map and crashed it. The lookup is now optional.

Drop this copy once an upstream release fixes it.

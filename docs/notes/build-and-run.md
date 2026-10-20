# Build & run (foundation)

## Library tests (platform-agnostic core)
```
cargo test
```
Runs `keel-core`, `keel-storage`, `keel-service` (the UI crate is wasm-only and excluded from the host suite).

## Desktop dev (Windows) — the v1 dev target
The Dioxus UI (`crates/ui`) is wasm; the Tauri shell (`crates/src-tauri`) serves it.

1. Build the UI to wasm:
   ```
   cd crates/ui
   dx build --platform web
   ```
   Output lands in `crates/ui/target/dx/keel-ui/debug/web/public`.
2. Stage it where Tauri's `frontendDist` (`../ui/dist`) expects it:
   ```
   rm -rf crates/ui/dist && mkdir crates/ui/dist
   cp -r crates/ui/target/dx/keel-ui/debug/web/public/* crates/ui/dist/
   ```
   (`dist/` is git-ignored; rebuild it from the step above.)
3. Run the desktop app:
   ```
   cd crates/src-tauri
   cargo tauri dev
   ```

> A `beforeDevCommand`/`beforeBuildCommand` can later automate steps 1–2.

## iOS (requires a Mac)
iOS cannot be built on Windows. On a Mac with Xcode:
```
cd crates/src-tauri
cargo tauri ios init
cargo tauri ios dev
```
The Rust core/storage/service and the Dioxus UI are platform-agnostic, so this is the only place the Apple toolchain is needed.

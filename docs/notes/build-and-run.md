# Build & run

## Library tests (platform-agnostic core)
```
cargo test
```
Runs `keel-core`, `keel-storage`, `keel-service` and the app crate. The app crate embeds the web UI at compile time, so build it once first (`cd web && pnpm build`).

## Desktop dev — the v1 dev target
The React UI (`web/`) is served by the Tauri shell (`crates/src-tauri`). First-time setup: Node 24, `corepack enable`, `cd web && pnpm install`, and from the repo root `cargo install wasm-bindgen-cli --version 0.2.129 --locked --root .tools`.

```
cd crates/src-tauri
cargo tauri dev
```
`beforeDevCommand` runs `pnpm dev` in `web/` (Vite with hot reload); `cargo tauri build` runs `pnpm build` and bundles `web/dist`.

## iOS (requires a Mac)
iOS cannot be built on Windows. On a Mac with Xcode:
```
cd crates/src-tauri
cargo tauri ios dev
```
The Xcode project (`gen/apple`) is committed; don't re-run `cargo tauri ios init`.

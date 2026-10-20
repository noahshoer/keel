# iOS notifications — Mac checklist

Everything except building and running on iOS was done on Windows. This is the one sitting on the Mac.

## 1. One-time setup (about 20 minutes)
1. Install **Xcode** from the App Store, open it once, and accept the license. Then install the command-line tools:
   ```bash
   xcode-select --install
   ```
2. Install the Rust iOS targets:
   ```bash
   rustup target add aarch64-apple-ios aarch64-apple-ios-sim
   ```
3. Install the Tauri CLI and the Dioxus CLI (same as on Windows):
   ```bash
   cargo install tauri-cli --version "^2"
   ```
   ```bash
   cargo install dioxus-cli
   ```
4. Clone the repo, then build the web UI once (Tauri loads it from `crates/ui/dist`):
   ```bash
   cd crates/ui && dx build --platform web && mkdir -p dist && cp -r target/dx/keel-ui/debug/web/public/* dist/
   ```
5. Generate the Xcode project (only works on macOS):
   ```bash
   cd crates/src-tauri && cargo tauri ios init
   ```
   Commit the generated `crates/src-tauri/gen/apple` folder.

## 2. Run it
- **Simulator:**
  ```bash
  cd crates/src-tauri && cargo tauri ios dev
  ```
  Pick an iPhone simulator when asked.
- **Your iPhone:** plug it in, enable Developer Mode (Settings → Privacy & Security), then run the same command and pick the phone. Xcode needs a signing team: open `gen/apple/*.xcodeproj` → target → Signing & Capabilities → pick your Apple ID team. (A free Apple ID works; the install expires after 7 days.)

## 3. What to check
- [ ] Settings (gear) → turn on **Task reminders** → iOS shows the "Allow notifications?" prompt → Allow → the switch stays on.
- [ ] **Send test notification** → a banner appears (pull down Notification Center if the app is in front).
- [ ] Add a task due 2 minutes from now (pick today and a time two minutes ahead in the date picker), then **close the app** (swipe it away). The reminder arrives on time.
- [ ] Complete a task that had a pending reminder → it no longer fires.
- [ ] Turn on **Daily summary**; the "Coming up" list shows it.
- [ ] Tapping a notification opens the app.
- [ ] If you deny permission: the switch stays off and the panel says notifications are blocked.

## Known limits (v1)
- Reminders are planned for the next 14 days, at most 60 at a time (iOS keeps 64). They re-plan every time you open the app or change a task, so far-off tasks get scheduled as they come into range.
- Tasks added on another device only get reminders on this phone after sync is turned on.

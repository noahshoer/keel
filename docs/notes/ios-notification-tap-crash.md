# iOS: tapping a notification crashed Keel

**Status (2026-10-21):** fix shipped on `feat/liquid-glass` (`2232917`). It has not yet been confirmed on the phone with a real cold-launch tap. If Noah reports the crash again, start here.

## Symptom
Tapping a Keel notification (the daily summary, or a task reminder) while Keel is fully closed. The app opens and crashes immediately. Tapping one while Keel is already running was not affected.

## Cause
A bug in `tauri-plugin-notification` 2.7.0 (iOS). `NotificationHandler.toActiveNotification` force-unwraps `notificationsMap[request.identifier]!`. That map is in memory and only holds notifications scheduled by the current run of the app. A tap on a notification scheduled by an earlier run cold-launches the app with an empty map, and the unwrap traps.

The crash report shows `EXC_BREAKPOINT` / `SIGTRAP`. Keel frames sit directly under `-[_UISceneUserNotificationsBSActionsHandler _respondToActions:forFBSScene:…]`. The first reports were `Keel-2026-10-21-063007.ips` and `…-070121.ips` on Noah's iPhone.

## Fix
The plugin is vendored at `crates/vendor/tauri-plugin-notification`, with the lookup made optional (search for `KEEL PATCH`). It's wired in through `[patch.crates-io]` in the root `Cargo.toml`. `KEEL_PATCH.md` in that folder records the change. Drop the copy once an upstream release fixes it.

The bug is written up for upstream in `docs/notes/upstream-notification-ios-issue.md` (second section). That issue is not filed yet.

## Verify
The simulator can't tap a local notification, so verification needs the phone:
1. Install a build (CLAUDE.md, "Keel runs on Noah's iPhone").
2. Create a task due in about 2 minutes, with a reminder "at time of task".
3. Swipe Keel away so it's fully closed.
4. Tap the notification when it arrives. Keel should open normally.

## If it crashes again
Pull the newest crash report and read the faulting thread:

```bash
D=00008160-00082D8921400036
xcrun devicectl device info files --device $D --domain-type systemCrashLogs | grep -i keel
xcrun devicectl device copy from --device $D --domain-type systemCrashLogs \
  --source Keel-<date>.ips --destination /tmp/keel.ips
```

The `.ips` file is one JSON header line followed by a JSON body. In the body, `faultingThread` indexes into `threads`, and each frame's `imageIndex` points into `usedImages`. To symbolicate Keel frames, use the dSYM from the same build (`crates/src-tauri/gen/apple/build/keel_iOS.xcarchive/dSYMs/Keel.app.dSYM`, overwritten by every build) with `atos -o <dSYM>/Contents/Resources/DWARF/Keel -l <load address> <address>`.

Things to check:
- Does the stack still pass through the notification plugin? Is the vendored patch still the one being built? `Cargo.lock` should show `tauri-plugin-notification` with no `source = "registry+…"` line.
- If the crash is elsewhere, the other iOS-only Swift in the app is the glass plugin (`crates/plugin-glass/ios`).

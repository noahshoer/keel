# Draft: tauri-apps/plugins-workspace issue

**Title:** [notification] iOS: Rust `pending()` and `cancel_all()` always fail, so scheduled notifications can't be cleared from Rust

### Describe the bug

On iOS, two of the mobile Rust APIs in `tauri-plugin-notification` 2.7.0 fail on every call:

1. **`pending()` always returns a deserialize error.** The Rust `PendingNotification` requires a `schedule` field (`src/models.rs:284-289`, `schedule: Schedule` with no `#[serde(default)]`). The iOS side encodes only `{ id, title, body }` (`ios/Sources/NotificationHandler.swift:90-103`), so parsing fails.
2. **`cancel_all()` always fails** with
   `Swift.DecodingError.valueNotFound(Swift.Dictionary<Swift.String, Any>, ... "Cannot get keyed decoding container -- found null value instead")`.
   `cancel_all` sends unit args, `run_mobile_plugin("cancel", ())` (`src/mobile.rs:193-195`), which serializes to `null`. Swift's `cancel` immediately calls `invoke.parseArgs(CancelArgs.self)` (`ios/Sources/NotificationPlugin.swift:217-218`), and that needs an object. `CancelArgs.notifications` is already optional and means "cancel everything" when missing, so an empty object `{}` would work.

As a result, a Rust app can't find out what's pending on iOS or clear it. Only `cancel(ids)` works, so the app has to keep its own record of the ids it scheduled. A common pattern breaks: cancel everything pending, then schedule the new set. The old requests stay, and notifications for items that were removed (for example, completed tasks) still fire.

### Reproduction

From Rust on an iOS simulator (iOS 18.6, Xcode 16.4), with notification permission granted:

```rust
use tauri_plugin_notification::{NotificationExt, Schedule};

let n = app.notification();
let date = time::OffsetDateTime::now_utc() + time::Duration::hours(1);
n.builder().id(1).title("t").schedule(Schedule::At { date, repeating: false, allow_while_idle: false }).show()?;

dbg!(n.pending());     // Err(...) - missing field `schedule`
dbg!(n.cancel_all());  // Err(... "found null value instead")
n.cancel(vec![1])?;    // works
```

### Expected behavior

- `pending()` returns the pending notifications. Either iOS includes `schedule` (it's already kept in `userInfo["__SCHEDULE__"]`), or the Rust field becomes `Option<Schedule>` / `#[serde(default)]`.
- `cancel_all()` removes all pending requests. Sending `{}` instead of `()` from Rust would be enough.

### Workaround

Persist the ids passed to `.id(...)` when scheduling, and call `cancel(ids)` before rescheduling.

### Related

- #3256: iOS `Schedule::At` ignores the time zone (parsed with a literal `'Z'` and no `timeZone`). We hit it in the same app; its workaround is to send the local wall clock labeled as UTC.
- #2341 and #3550 report similar `pending()` / `cancelAll()` failures, but on Android, with different causes.

### Environment

- `tauri-plugin-notification` 2.7.0 (Rust API, no JS)
- tauri-cli 2.12.3, Xcode 16.4, iOS 18.6 simulator, macOS (arm64)

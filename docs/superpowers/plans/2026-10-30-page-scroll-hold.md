# The page holds still under the task view (iOS)

**Why:** Focusing the title or description in a medium task sheet made WebKit scroll the whole page up (136–183 px in the phone's diag log, 2026-10-30) to reveal the field, so the list under the sheet moved before the sheet went large. The list and the sheet are one page with one scroll, and WebKit has no switch for its reveal. A first try had the page own the keyboard (WebKit's keyboard observers removed, the app moving its own content); Noah found "+ Add task" clunky with it (the field moved late, then fast), so WebKit keeps its keyboard handling and only the page's scroll is held, only while the task view is open.

**Approach:**

- **Swift (`crates/plugin-ios`, `PageScroll.swift`):** `PageScroll.hold(true)` observes the web view's scroll view and puts any change of its offset back inside the same change, before it's drawn; `hold(false)` stops. The plugin command is `pageScroll` (`{ on }`). The task sheet's own scrolling (`.tv-scroll`) and its drag (touch handlers moving the sheet) don't use the page's scroll, so both work as before.
- **Rust:** `tauri_plugin_keel_ios::hold_page_scroll`, behind the async command `hold_page_scroll` (`HoldPageScrollArgs { on }`); a no-op off iOS.
- **Page (`web/src/app/App.tsx`):** the frame sends `hold_page_scroll` when the task view opens or closes (`viewOpen`), only on a change. It opens on a tap, well before a field in it is focused.

**Tests:** Vitest: opening the view holds the page (mock state), closing frees it. Simulator: `scripts/sim-checks/page-scroll-hold.js` asks the page to scroll with the view open and closed; the diag log's `SV offset` stays 0,0 while held and goes to 0,200 once free.

**Phone check (Noah):** with the task view at medium, tap the title, then the description: the list under it doesn't move, and the sheet goes large above the keyboard. With it closed, "+ Add task" moves with the keyboard as before. Swiping the sheet down with the keyboard up still closes it.

## As built

- Simulator (React build, iPhone 17): `page scroll held` / `page scroll free` in the diag log as the view opens and closes; `SV offset 0.0,0.0` while held, `0.0,200.0` once free.
- A scroll asked for from the page itself (`window.scrollTo`) still moves the page's own scroll position while held (the web view's stays put); the app never does that, and WebKit's keyboard reveal moves the web view's.

- The first keyboard after launch comes with no pace (`KB WillShowNotification … dur 0.0` in the phone log, `DidShow` 2 ms later), so the sheet jumped to large instead of sliding. Swift now sends the usual pace for a keyboard coming up that reports none (the last pace seen, or iOS's 0.3833 s, at Keel's 0.9 speed: 0.426 s), so the sheet slides with it; Noah found the sheet's own 0.3 s too fast. The page's fallback for a pace of 0 (its own settle, 0.3 s) stays for safety; `bottom` only transitions with a pace.

Phone check (Noah, 2026-10-30): passed. The list stays put under the sheet, and the first keyboard after launch slides the sheet at keyboard speed.

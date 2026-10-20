# Native Liquid Glass on iPhone

**Status:** approved by Noah, 2026-10-20.

## Why

On the iPhone, Keel is a web view, and the places where you touch it most give that away: the task composer, the text boxes and the small buttons. iOS 26 apps (Todoist was the reference Noah showed) use Liquid Glass for exactly these things: a floating glass card for a new task, one row of glass chips that scrolls sideways, and round glass buttons over the content. Noah wants Keel to feel like that. In his words: "task creation, and input text boxes and any little buttons like hamburger menus", with "a scroll through Liquid Glass buttons".

Keel's minimum is already iOS 26 (`bundle.iOS.minimumSystemVersion`), so `.glassEffect()` can be used without fallbacks. CI already builds the Swift side (`ios` job).

## Goals

1. **Composer (new task)** — mockup A in `.superpowers/brainstorm/mockups/quick-add-v2.html` (git-ignored; open it on the Mac):
   - A green glass **+** floats bottom right on every task list. Tapping it shows the keyboard with a glass card resting directly on it.
   - The card has a title line (quick-add highlighting live as you type, same colors and rules as today), then a smaller description line.
   - Below the text is one horizontally scrolling row of glass chips: Date, Priority, Labels, Reminders, Deadline, Repeat, Project/section.
     - A chip with a value shows it ("Tomorrow 09:00", "P1", "#Work"). An empty chip shows its name.
     - There is no "+ (more)" chip, since every option already has its own chip (my call).
   - A green send button is pinned at the right end of the row and never scrolls away.
   - **Typed phrases and chips (my call, Todoist's behavior):**
     - Quick add fills chips as it recognizes things.
     - Picking a value on a chip removes the matching typed phrase from the title, so the chip wins and the title never disagrees with the chips.
     - The save then sees one consistent set of values.
   - **After sending:** the card stays open and empty for the next task, as the web composer does today (`keep_open`).
   - **Closing:** swiping the card down or tapping outside it discards what was typed.
   - **Where it adds:**
     - The floating + uses the current view's defaults (project, date, label), like the web header's add.
     - The in-list "Add task" rows (per section, per day in Upcoming) stay on iOS and open the same native composer with their section or date (my call).
   - Adding a sub-task opens the same composer, aimed at the parent.
2. **Editor (existing task)** — Noah: "editing should also get liquid glass". Tapping a task opens the same card filled in with the task's title, description and chip values.
   - The send button becomes a checkmark.
   - **No quick-add parsing while editing** (my call), the same as the web editor (`quick_add: false`). Typing "p1" into an existing title stays text, and the chips are how you change values. So the edit card's title has no highlighting.
   - Swiping the card down or tapping outside it cancels without saving (my call).
3. **Pickers:** tapping a chip opens a native iOS picker:
   - Date gets a calendar plus the time wheel.
   - Priority gets a short glass menu.
   - Labels, Reminders, Deadline, Repeat and Project/section each get a native sheet or menu.
   - Every picker offers the same choices as the current web picker; nothing is dropped.
4. **Small buttons:**
   - The hamburger becomes a round glass button, top left, floating over the list.
   - A round glass search button sits top right (my call). There's no ••• menu, because the web header has no actions to put in one today.
5. **Navigation stays the sidebar drawer.** Noah explicitly chose it over a bottom tab bar. The swipe gestures (`ui::gestures`) keep working.

## Non-goals

- **No tab bar** (Noah's choice).
- **The drawer itself stays web**, along with the task list, settings, search results, filters and everything else not listed above. They can come later.
- **No change on Windows or desktop.** They keep the current web composer and editor.
- Android, iPad-specific layouts, voice input (Todoist has a mic button; Keel doesn't need one).

## Anti-goals (it fails even if it works if…)

- **iPhone and Windows read the same typed text differently.** Quick add must stay the single Rust parser in `keel-core::quickadd`, with no Swift reimplementation.
- **Saving behaves differently from the web editor**: recurrence, reminders, temp ids and the command log must go through the same code path.
- **It looks like web chrome painted to resemble glass.** The glass must be real SwiftUI `.glassEffect()`.
- **The keyboard opens a black gap or scrolls the page** (the WKWebView bug that bit the time field). The native card must sit on the keyboard with the web view untouched.
- **Touches stop reaching the list** in the areas the overlay doesn't cover.

## Constraints

- Tauri v2 mobile plugin: a Rust crate plus a Swift package, in the workspace, e.g. `crates/plugin-glass` (name is the builder's call).
  - Swift builds only on the Mac. CI (`macos-26`) must keep passing, and the Windows job must keep building (the plugin's Rust side compiles to no-ops on desktop).
- The UI is Dioxus/wasm and already links `keel-core` (quick add, recurrence text, models). The Tauri crate links `keel-core` and `keel-service`.
- Repo is public; nothing secret goes in.
- Work style (CLAUDE.md): a plan in `docs/superpowers/plans/`, a feature branch, TDD for the Rust/wasm logic, a commit per task, merge `--no-ff`, push.

## How it fits together (my call; the builder may refine it)

**The web UI stays the brain; the Swift side is a view it drives.**
- Dioxus decides when the composer or editor opens and what it shows: title, description, chip labels, highlight spans, the lists for pickers (projects, sections, labels).
- Dioxus sends that to Swift through plugin commands.
- Swift sends user actions back as plugin events: text changed, chip tapped, picker value chosen, submit, cancel.
- Dioxus parses each text change with `keel-core::quickadd` (as it does today), then pushes back the new spans and chip labels.
- On submit, Dioxus saves through the same functions the web editor uses (`editor::add_from_editor`, `editor::changes` + `api::update_task`, `editor::sync_reminders`).

This keeps one parser, one save path, and a Swift side that holds no task logic.

**Overlay:**
- The plugin's `load(webview:)` adds a SwiftUI host view over the WKWebView.
- The host view is transparent to touches everywhere except its glass controls, so the list underneath stays scrollable and tappable.
- The floating buttons (+, hamburger, top-right capsule) live in the same overlay. Their web versions are hidden on iOS only.
- **Detecting iOS** (to hide the web composer/buttons and use the native path): a command like the existing `reminders_when_closed`, or a plugin `available` check.

**Highlighting in a native text field:**
- Swift renders the spans Dioxus sends (ranges plus kind: date, project, label, priority…) as attributed-text colors in the title field.
- Latency should be imperceptible. If round-tripping each keystroke through the web view isn't, debounce or move parsing to a Rust call on the Swift side. Either way it must be the same `keel-core` parser.

## Open risk (spike first)

The plan should start with a throwaway spike on the simulator: a SwiftUI glass button over the Tauri web view that passes touches through elsewhere, plus a native text field on the keyboard, round-tripping text to Dioxus and spans back. If touch pass-through or keyboard layout doesn't work cleanly, **stop and go back to Noah** with what failed. Don't fall back on your own to native buttons only or to a card without highlighting (Noah's choice).

## Left to the builder

- Plugin crate and Swift file layout, command/event names and payload shapes.
- Exact glass styling: tints, spacing, chip order beyond the list above, animations. Match the mockup and Keel's green `#268366`.
- How each non-date picker looks natively, as long as it offers the same choices as the web one.
- Testing split:
  - Rust/wasm logic (the state Dioxus sends, the mapping from Swift events to edits) is unit tested.
  - Swift views are checked by hand on the simulator and on Noah's phone.

//! One stylesheet following Todoist's layout and palette, softened: rounded
//! corners, a floating main pane, gentle shadows and motion. Light and dark
//! themes are CSS variables. (Own CSS; no Todoist assets.)
//!
//! Theme resolution: `data-theme="light|dark"` on <html> wins; with no
//! attribute ("System") the OS `prefers-color-scheme` decides.

use base64::Engine;

/// Inter (variable weight, SIL OFL — see assets/fonts/Inter-OFL.txt), compiled
/// into the wasm and served as data URLs so the app looks the same offline
/// (no asset pipeline needed).
const INTER_LATIN: &[u8] = include_bytes!("../assets/fonts/inter-latin-wght-normal.woff2");
const INTER_LATIN_EXT: &[u8] = include_bytes!("../assets/fonts/inter-latin-ext-wght-normal.woff2");

/// `@font-face` rules for the bundled Inter subsets (encoded once).
fn fonts() -> &'static str {
    static FONTS: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    FONTS.get_or_init(font_faces)
}

fn font_faces() -> String {
    let face = |bytes: &[u8], range: &str| {
        let data = base64::engine::general_purpose::STANDARD.encode(bytes);
        format!(
            "@font-face {{ font-family: \"Inter\"; font-style: normal; font-weight: 100 900; \
             font-display: swap; src: url(\"data:font/woff2;base64,{data}\") format(\"woff2\"); \
             unicode-range: {range}; }}\n"
        )
    };
    face(
        INTER_LATIN_EXT,
        "U+0100-02BA, U+02BD-02C5, U+02C7-02CC, U+02CE-02D7, U+02DD-02FF, U+0304, U+0308, U+0329, \
         U+1D00-1DBF, U+1E00-1E9F, U+1EF2-1EFF, U+2020, U+20A0-20AB, U+20AD-20C0, U+2113, U+2C60-2C7F, U+A720-A7FF",
    ) + &face(
        INTER_LATIN,
        "U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+0304, U+0308, U+0329, \
         U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD",
    )
}

const LIGHT: &str = r#"
  color-scheme: light;
  --app-bg: #f5f3f0;
  --bg: #ffffff;
  --sidebar: transparent;
  --text: #202020;
  --muted: #8a8a8a;
  --subtle: #666666;
  --divider: #efefef;
  --hover: rgba(0,0,0,.045);
  --row-hover: #f7f5f3;
  --icon-hover: rgba(0,0,0,.06);
  --active-bg: #ffffff;
  --active-text: #1d6b53;
  --border: #e3e3e3;
  --border-strong: #c9c9c9;
  --chip-border: #e8e8e8;
  --chip-bg: #ffffff;
  --chip-hover: #f6f6f6;
  --chip-text: #444444;
  --panel: #f8f6f4;
  --btn-cancel: #f2f2f2;
  --btn-cancel-hover: #e7e7e7;
  --btn-cancel-text: #444444;
  --accent: #268366;
  --accent-hover: #1e6b53;
  --focus-ring: rgba(38,131,102,.2);
  --shadow-sm: 0 1px 2px rgba(0,0,0,.06), 0 1px 3px rgba(0,0,0,.04);
  --shadow-md: 0 4px 14px rgba(0,0,0,.07), 0 1px 3px rgba(0,0,0,.05);
  --shadow-lg: 0 12px 32px rgba(0,0,0,.12), 0 2px 6px rgba(0,0,0,.06);
  --p1: #d1453b; --p1-bg: rgba(209,69,59,.10);
  --p2: #eb8909; --p2-bg: rgba(235,137,9,.10);
  --p3: #246fe0; --p3-bg: rgba(36,111,224,.10);
  --p4: #b8b8b8;
  --due-overdue: #d1453b;
  --due-today: #058527;
  --due-tomorrow: #ad6200;
  --due-week: #692fc2;
  --due-later: #808080;
  --streak: #c2410c;
  --inbox: #246fe0;
  --today: #058527;
  --upcoming: #692fc2;
"#;

const DARK: &str = r#"
  color-scheme: dark;
  --app-bg: #161616;
  --bg: #212121;
  --sidebar: transparent;
  --text: #e8e8e8;
  --muted: #9a9a9a;
  --subtle: #b0b0b0;
  --divider: #303030;
  --hover: rgba(255,255,255,.06);
  --row-hover: #292929;
  --icon-hover: rgba(255,255,255,.08);
  --active-bg: #2b2b2b;
  --active-text: #6fcfaa;
  --border: #3a3a3a;
  --border-strong: #585858;
  --chip-border: #3a3a3a;
  --chip-bg: #262626;
  --chip-hover: #303030;
  --chip-text: #d0d0d0;
  --panel: #272727;
  --btn-cancel: #2f2f2f;
  --btn-cancel-hover: #3a3a3a;
  --btn-cancel-text: #e0e0e0;
  --accent: #2a9474;
  --accent-hover: #35ad88;
  --focus-ring: rgba(53,173,136,.25);
  --shadow-sm: 0 1px 2px rgba(0,0,0,.35);
  --shadow-md: 0 4px 14px rgba(0,0,0,.35), 0 1px 3px rgba(0,0,0,.3);
  --shadow-lg: 0 12px 32px rgba(0,0,0,.5), 0 2px 6px rgba(0,0,0,.35);
  --p1: #ff7066; --p1-bg: rgba(255,112,102,.14);
  --p2: #ff9a14; --p2-bg: rgba(255,154,20,.14);
  --p3: #5297ff; --p3-bg: rgba(82,151,255,.14);
  --p4: #6b6b6b;
  --due-overdue: #ff7066;
  --due-today: #25b84c;
  --due-tomorrow: #ff9a14;
  --due-week: #a970ff;
  --due-later: #9a9a9a;
  --streak: #ff9a5a;
  --inbox: #5297ff;
  --today: #25b84c;
  --upcoming: #a970ff;
"#;

const RULES: &str = r#"
:root { --r-sm: 6px; --r: 8px; --r-md: 10px; --r-lg: 14px; --ease: cubic-bezier(.2,.8,.2,1); }
* { box-sizing: border-box; }
html, body { margin: 0; height: 100%; }
body {
  font-family: "Inter", "Segoe UI Variable Text", "Segoe UI Variable", -apple-system, BlinkMacSystemFont,
    "SF Pro Text", system-ui, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  font-size: 14px; color: var(--text); background: var(--app-bg);
  -webkit-font-smoothing: antialiased; letter-spacing: -0.006em;
  font-feature-settings: "cv11", "ss03"; font-optical-sizing: auto;
}
button, input, select { font-family: inherit; color: inherit; }
button, .chip, .nav-item, .icon-btn, .btn, .check, .add-task-row, .view-toggle, .label-opt, .task-row {
  transition: background-color .16s var(--ease), color .16s var(--ease), border-color .16s var(--ease),
    box-shadow .16s var(--ease), transform .12s var(--ease), opacity .16s var(--ease);
}
@keyframes pop-in { from { opacity: 0; transform: translateY(-4px) scale(.98); } to { opacity: 1; transform: none; } }
@keyframes fade-up { from { opacity: 0; transform: translateY(3px); } to { opacity: 1; transform: none; } }

.app { display: flex; height: 100vh; background: var(--app-bg); }

/* Sidebar (sits on the app background; the main pane floats beside it) */
.sidebar { width: 272px; flex-shrink: 0; background: var(--sidebar); padding: 14px 10px 14px 12px; overflow-y: auto; }
.brand { display: flex; align-items: center; gap: 10px; font-weight: 700; font-size: 15px; padding: 6px 8px 18px; }
.brand-mark { width: 28px; height: 28px; flex-shrink: 0; display: flex; color: var(--accent); filter: drop-shadow(0 1px 1.5px rgba(0,0,0,.12)); }
.brand .theme-toggle { margin-left: auto; }
.nav-item { display: flex; align-items: center; gap: 8px; height: 36px; padding: 0 10px; border-radius: var(--r-md);
  cursor: pointer; color: var(--text); user-select: none; margin-bottom: 1px; }
@media (hover: hover) { .nav-item:hover { background: var(--hover); } }
.nav-item.pressed { background: var(--hover); }
.nav-item:active { transform: scale(.985); }
.nav-item.active { background: var(--active-bg); color: var(--active-text); box-shadow: var(--shadow-sm); font-weight: 500; }
.nav-item .count { margin-left: auto; color: var(--muted); font-size: 12px; }
.nav-item.active .count { color: var(--active-text); }
.nav-icon { width: 24px; height: 24px; display: flex; align-items: center; justify-content: center; }
.nav-icon.inbox { color: var(--inbox); }
.nav-icon.today { color: var(--today); }
.nav-icon.upcoming { color: var(--upcoming); }
.nav-icon.hash { color: var(--muted); font-size: 16px; font-weight: 500; }
.section-head { display: flex; align-items: center; justify-content: space-between;
  padding: 20px 10px 6px; font-weight: 600; color: var(--muted); font-size: 12px;
  text-transform: uppercase; letter-spacing: .04em; }
.project-form { padding: 4px 8px; }
.project-input { width: 100%; padding: 8px 10px; border: 1px solid var(--border); border-radius: var(--r);
  font-size: 13px; outline: none; background: var(--bg); transition: border-color .16s, box-shadow .16s; }
.project-input:focus { border-color: var(--border-strong); box-shadow: 0 0 0 3px var(--focus-ring); }

.icon { display: inline-flex; align-items: center; justify-content: center; }
.icon-btn { border: none; background: none; cursor: pointer; color: var(--muted); padding: 5px; border-radius: var(--r);
  display: inline-flex; align-items: center; justify-content: center; }
.icon-btn:hover { background: var(--icon-hover); color: var(--text); }
.icon-btn:active { transform: scale(.92); }

/* Main pane: a rounded card floating on the app background */
.main { flex: 1; overflow-y: auto; background: var(--bg); margin: 8px 8px 8px 0;
  border-radius: var(--r-lg); box-shadow: var(--shadow-md); }
.content { max-width: 820px; margin: 0 auto; padding: 40px 48px 96px; }
/* A view fades in when it lands; two identical animations alternate so a
   switch restarts it (main.rs `swap`). */
@keyframes view-in-a { from { opacity: 0; } to { opacity: 1; } }
@keyframes view-in-b { from { opacity: 0; } to { opacity: 1; } }
.content.view-in-a { animation: view-in-a .2s var(--ease); }
.content.view-in-b { animation: view-in-b .2s var(--ease); }
.view-title { font-size: 26px; font-weight: 700; margin: 0 0 2px; letter-spacing: -0.02em; }
.view-sub { color: var(--muted); font-size: 12px; margin: 0 0 12px; }
.section-title { display: flex; align-items: baseline; gap: 6px; font-weight: 650; font-size: 14px;
  padding: 22px 10px 6px; }
.section-title .muted { color: var(--muted); font-weight: 400; font-size: 13px; }

/* Task rows: a soft list, rounded highlight on hover */
.task-slot { display: contents; }
.task { border-radius: var(--r-md); }
.task-row { display: flex; align-items: flex-start; gap: 12px; padding: 10px 10px 9px; border-radius: var(--r-md); }
@media (hover: hover) { .task:not(.completed) .task-row:hover { background: var(--row-hover); } }
.task.pressed .task-row { background: var(--row-hover); }
.check { width: 19px; height: 19px; min-width: 19px; margin-top: 1px; border-radius: 50%; padding: 0;
  border: 1.5px solid var(--p4); background: transparent; color: var(--muted); cursor: pointer;
  display: flex; align-items: center; justify-content: center; }
.check .icon { opacity: 0; transform: scale(.6); transition: opacity .16s var(--ease), transform .16s var(--ease); }
@media (hover: hover) {
  .check:hover .icon { opacity: 1; transform: scale(1); }
  .check:hover { background: var(--hover); }
}
.check:active { transform: scale(.82); }
.check.p1 { border: 2px solid var(--p1); background: var(--p1-bg); color: var(--p1); }
.check.p2 { border: 2px solid var(--p2); background: var(--p2-bg); color: var(--p2); }
.check.p3 { border: 2px solid var(--p3); background: var(--p3-bg); color: var(--p3); }
.task-body { flex: 1; min-width: 0; }
.task-content { line-height: 20px; word-wrap: break-word; }
.task-desc { color: var(--muted); font-size: 12px; line-height: 16px; margin-top: 2px; }
.task-meta { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; margin-top: 5px; font-size: 12px; }
.due { display: inline-flex; align-items: center; gap: 4px; }
.due.overdue { color: var(--due-overdue); }
.due.today { color: var(--due-today); }
.due.tomorrow { color: var(--due-tomorrow); }
.due.week { color: var(--due-week); }
.due.later { color: var(--due-later); }
.streak { color: var(--streak); font-weight: 500; }
.task-project { font-size: 12px; color: var(--muted); white-space: nowrap; align-self: flex-end; padding-bottom: 2px; }
.task-actions { display: flex; gap: 2px; opacity: 0; transform: translateX(4px); }
.task-actions { transition: opacity .16s var(--ease), transform .16s var(--ease); }
/* Mouse: hover shows a task's actions. Touch: a long press does (gestures.rs);
   a hover reveal on iPhone would cost every tap on a task an extra tap. */
@media (hover: hover) { .task:hover .task-actions { opacity: 1; transform: none; pointer-events: auto; } }
.task.pressed .task-actions { opacity: 1; transform: none; }
.streak-panel { display: flex; flex-wrap: wrap; align-items: center; gap: 18px; margin: 2px 0 10px 40px;
  padding: 12px 14px; background: var(--panel); border-radius: var(--r-lg); font-size: 13px;
  animation: pop-in .18s var(--ease); }
.streak-panel .stat b { font-size: 15px; }
.streak-panel input { width: 64px; padding: 5px 8px; border: 1px solid var(--border); border-radius: var(--r);
  background: var(--bg); outline: none; }
.streak-panel input:focus { border-color: var(--border-strong); box-shadow: 0 0 0 3px var(--focus-ring); }

/* Add task */
.add-task-row { display: flex; align-items: center; gap: 12px; padding: 9px 10px; color: var(--muted);
  cursor: pointer; border-radius: var(--r-md); }
.add-task-row .plus { width: 19px; height: 19px; border-radius: 50%; color: var(--accent);
  display: flex; align-items: center; justify-content: center; transition: background-color .16s, color .16s; }
/* Mouse only: on iPhone a hover style makes WebKit spend the first tap on hover. */
@media (hover: hover) {
  .add-task-row:hover { color: var(--accent); background: var(--row-hover); }
  .add-task-row:hover .plus { background: var(--accent); color: #fff; }
}
.editor { border: 1px solid var(--border); border-radius: var(--r-lg); padding: 12px 12px 0; margin: 8px 0;
  background: var(--bg); box-shadow: var(--shadow-md); animation: pop-in .18s var(--ease);
  transition: border-color .16s, box-shadow .16s; }
.editor:focus-within { border-color: var(--border-strong); box-shadow: var(--shadow-lg); }
.editor .name { width: 100%; border: none; outline: none; font-size: 15px; font-weight: 600; padding: 2px 0; background: transparent; }
.editor .desc { width: 100%; border: none; outline: none; font-size: 13px; padding: 5px 0 12px; background: transparent; }
.editor .name::placeholder, .editor .desc::placeholder, .project-input::placeholder { color: var(--muted); }
.chips { display: flex; flex-wrap: wrap; gap: 8px; padding-bottom: 12px; }
.chip { display: inline-flex; align-items: center; gap: 5px; height: 30px; padding: 0 10px;
  border: 1px solid var(--chip-border); border-radius: var(--r); font-size: 12px; color: var(--subtle); background: var(--chip-bg); }
.chip:hover { background: var(--chip-hover); border-color: var(--border-strong); }
.chip:focus-within { box-shadow: 0 0 0 3px var(--focus-ring); }
.chip input, .chip select { border: none; outline: none; background: transparent; font-size: 12px; color: var(--chip-text); }
.quick-preview { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; padding: 4px 0 6px; font-size: 12px; }
.quick-chip { padding: 2px 8px; border-radius: 999px; background: var(--panel); color: var(--chip-text);
  animation: pop-in .16s var(--ease); }
.quick-chip.chip-date { color: var(--due-today); }
.quick-chip.chip-repeat { color: var(--due-week); }
.quick-chip.chip-priority { color: var(--p1); }
.quick-chip.chip-project { color: var(--p3); }
.quick-chip.chip-label { color: var(--p2); }
.chip select option { background: var(--chip-bg); color: var(--chip-text); }
.editor-footer { display: flex; justify-content: flex-end; gap: 8px; border-top: 1px solid var(--divider); padding: 10px 0 12px; }
.btn { border: none; border-radius: var(--r); padding: 8px 14px; font-size: 13px; font-weight: 600; cursor: pointer; }
.btn:active:not(:disabled) { transform: scale(.97); }
.btn-cancel { background: var(--btn-cancel); color: var(--btn-cancel-text); }
.btn-cancel:hover { background: var(--btn-cancel-hover); }
.btn-add { background: var(--accent); color: #fff; box-shadow: var(--shadow-sm); }
.btn-add:hover { background: var(--accent-hover); box-shadow: var(--shadow-md); }
.btn-add:disabled { opacity: .45; cursor: not-allowed; box-shadow: none; }

/* Labels + editor pickers */
.task-body { cursor: pointer; }
.label-tag { display: inline-flex; align-items: center; gap: 3px; color: var(--muted); }
.nav-icon.label { color: var(--muted); }
.chip-hash { color: var(--muted); font-weight: 600; }
.chip-btn { cursor: pointer; font-family: inherit; max-width: 220px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.chip-wrap { position: relative; }
.label-pop { min-width: 230px; }

/* Popovers (scheduler, menus, labels): one open per editor; a transparent
   backdrop behind them closes on click. */
.pop-backdrop { position: fixed; inset: 0; z-index: 19; }
.popover { position: absolute; top: calc(100% + 6px); left: 0; z-index: 20; padding: 6px;
  background: var(--bg); border: 1px solid var(--border); border-radius: var(--r-lg);
  box-shadow: var(--shadow-lg); animation: pop-in .16s var(--ease); transform-origin: top left; }
.menu { min-width: 210px; max-height: 320px; overflow-y: auto; }
.menu-item { display: flex; align-items: center; gap: 10px; width: 100%; padding: 7px 8px; border: none;
  background: none; border-radius: var(--r); cursor: pointer; font-size: 13px; text-align: left; color: var(--text); }
.menu-item:hover { background: var(--hover); }
.menu-item.indent { padding-left: 30px; }
.menu-item.checked { font-weight: 600; }
.menu-icon { display: inline-flex; color: var(--muted); }
.menu-label { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.menu-hint { color: var(--muted); font-size: 12px; }
.menu-check { display: inline-flex; color: var(--accent); }
.chip-open { border-color: var(--border-strong); background: var(--chip-hover); }
.chip-icon { display: inline-flex; }
.chip-caret { display: inline-flex; color: var(--muted); margin-left: 1px; }
.pri-1 { color: var(--p1); } .pri-2 { color: var(--p2); } .pri-3 { color: var(--p3); } .pri-4 { color: var(--muted); }
.loc-inbox { color: var(--inbox); } .loc-project, .loc-section { color: var(--muted); }
.due-chip.overdue { color: var(--due-overdue); } .due-chip.today { color: var(--due-today); }
.due-chip.tomorrow { color: var(--due-tomorrow); } .due-chip.week { color: var(--due-week); }
.due-chip.later { color: var(--due-later); }

/* Scheduler */
.scheduler { width: 280px; padding: 0; overflow: hidden; }
.sched-head { padding: 12px 14px; font-weight: 600; font-size: 13px; border-bottom: 1px solid var(--divider); }
.sched-quick { padding: 6px; border-bottom: 1px solid var(--divider); }
.q-today { color: var(--due-today); } .q-tomorrow { color: var(--due-tomorrow); }
.q-weekend { color: var(--p3); } .q-next { color: var(--due-week); } .q-none { color: var(--muted); }
.sched-cal { padding: 8px 10px 6px; }
.cal-nav { display: flex; align-items: center; gap: 2px; padding: 0 2px 6px; }
.cal-title { flex: 1; font-weight: 600; font-size: 13px; }
.cal-nav .icon-btn:disabled { opacity: .3; cursor: default; background: none; }
.cal-dot { width: 7px; height: 7px; border-radius: 50%; border: 1.5px solid currentColor; }
.cal-grid { display: grid; grid-template-columns: repeat(7, 1fr); gap: 2px; }
.cal-dow { text-align: center; font-size: 11px; color: var(--muted); padding: 2px 0 4px; }
.cal-day { height: 32px; border: none; background: none; border-radius: 50%; cursor: pointer; font-size: 12.5px;
  color: var(--text); font-variant-numeric: tabular-nums; }
.cal-day:hover { background: var(--hover); }
.cal-day.other { color: var(--muted); opacity: .6; }
.cal-day.past { color: var(--muted); opacity: .45; }
.cal-day.is-today { color: var(--accent); font-weight: 700; }
.cal-day.selected { background: var(--accent); color: #fff; font-weight: 600; opacity: 1; }
.sched-time { display: flex; align-items: center; gap: 8px; padding: 8px 12px 10px; border-top: 1px solid var(--divider); color: var(--muted); }
.time-input { flex: 1; min-width: 0; border: 1px solid var(--border); border-radius: var(--r); padding: 6px 8px;
  font-size: 12.5px; outline: none; background: var(--bg); color: var(--text); }
.time-input:focus { border-color: var(--border-strong); box-shadow: 0 0 0 3px var(--focus-ring); }
.time-native { position: relative; flex: 1; min-width: 0; display: flex; }
.time-input.native { -webkit-appearance: none; appearance: none; min-height: 36px; background: var(--bg); text-align: left; }
.time-input.native::-webkit-date-and-time-value { text-align: left; }
.time-hint { position: absolute; left: 9px; top: 50%; transform: translateY(-50%); color: var(--muted);
  font-size: 12.5px; pointer-events: none; }
.time-input.bad { border-color: var(--due-overdue); box-shadow: 0 0 0 3px rgba(209,69,59,.15); }

/* Title with highlighted quick-add phrases: textarea and mirror share one
   grid cell, so the mirror's wrapped text sizes the field. */
.title-wrap { display: grid; }
.title-wrap > .title-mirror, .title-wrap > .title-input { grid-area: 1 / 1; margin: 0; padding: 2px 0;
  font: inherit; font-size: 15px; font-weight: 600; line-height: 22px; letter-spacing: inherit;
  white-space: pre-wrap; overflow-wrap: anywhere; border: none; }
.title-mirror { color: transparent; pointer-events: none; user-select: none; }
.title-input { resize: none; overflow: hidden; background: transparent; outline: none; color: var(--text); }
.hl { color: transparent; border-radius: 4px; box-shadow: 0 0 0 2px var(--hl-bg); background: var(--hl-bg); }
.hl-date { --hl-bg: rgba(5,133,39,.14); }
.hl-repeat { --hl-bg: rgba(105,47,194,.14); }
.hl-priority { --hl-bg: rgba(209,69,59,.14); }
.hl-project { --hl-bg: rgba(36,111,224,.14); }
.hl-label { --hl-bg: rgba(235,137,9,.16); }
.label-opt { display: flex; align-items: center; gap: 8px; padding: 7px 8px; border-radius: var(--r); cursor: pointer; font-size: 13px; }
.label-opt:hover { background: var(--hover); }
.label-empty { color: var(--muted); font-size: 12px; padding: 6px 8px; }
.label-new { width: 100%; margin-top: 6px; padding: 7px 8px; font-size: 12px; border: 1px solid var(--border);
  border-radius: var(--r); outline: none; background: var(--bg); }
.label-new:focus { border-color: var(--border-strong); box-shadow: 0 0 0 3px var(--focus-ring); }

/* Completed tasks + sidebar management */
.view-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
/* iPhone: native glass buttons float over the header (menu left, search
   right) and the list (+ bottom right); keep their spots clear. */
.native-glass .menu-btn { display: none; }
.native-glass .content { padding-top: calc(env(safe-area-inset-top) + 60px); padding-bottom: 120px; }
/* Touch: Todoist-sized rows (a little shorter) with a bigger check circle,
   whose tap area reaches past it to ~44px. */
@media (pointer: coarse) {
  .task-row { gap: 14px; padding: 14px 8px 13px; }
  .task-content { font-size: 17px; line-height: 23px; }
  .task-desc { font-size: 14px; line-height: 19px; margin-top: 3px; }
  .task-meta { font-size: 14px; margin-top: 6px; gap: 12px; }
  .check { position: relative; width: 24px; height: 24px; min-width: 24px; margin-top: 0; border-width: 1.75px; }
  .check::after { content: ''; position: absolute; inset: -10px; }
  .task-project { font-size: 13px; }
  /* The add row, Reminders-style: a blank next task with a dotted circle,
     lined up with the task circles above it. */
  .add-task-row { padding: 14px 8px 13px; gap: 10px; font-size: 17px; }
  .add-task-row .plus { box-sizing: border-box; width: 24px; height: 24px; min-width: 24px;
    background: transparent; border: 2px dotted color-mix(in srgb, var(--text) 35%, transparent); }
  .add-task-row .plus svg { display: none; }
}
/* Touch: a long press shows a task's actions; no text selection or callout. */
@media (hover: none) { .task-row, .nav-item { -webkit-user-select: none; user-select: none; -webkit-touch-callout: none; } }
.view-head .view-title { flex: 1; min-width: 0; overflow-wrap: anywhere; }
.view-head .view-toggle { flex-shrink: 0; }
.menu-btn { display: none; margin-left: -6px; color: var(--subtle); }
.view-toggle { display: inline-flex; align-items: center; gap: 6px; border: none; background: none; cursor: pointer;
  color: var(--muted); font-size: 13px; padding: 6px 10px; border-radius: var(--r); }
.view-toggle:hover { background: var(--icon-hover); color: var(--text); }
.check.done { background: var(--muted); border: 1.5px solid var(--muted); color: var(--bg); }
.check.done .icon { opacity: 1; transform: none; }
.task.completed { animation: fade-up .2s var(--ease); }
.task.completed .task-content { text-decoration: line-through; color: var(--muted); }
.task.completed .task-body { cursor: default; }
.done-when { color: var(--muted); }
.nav-name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.nav-actions { display: none; gap: 0; }
/* Mouse: hover shows a row's rename/delete. Touch: a long press opens a menu
   (gestures.rs); a hover reveal would cost every tap an extra tap. */
@media (hover: hover) { .nav-item:hover .nav-actions { display: inline-flex; } }
.nav-actions .icon-btn { padding: 3px; }
.confirm-box { margin: 4px 0; padding: 10px 12px; border: 1px solid var(--border); border-radius: var(--r-md);
  background: var(--bg); box-shadow: var(--shadow-md); animation: pop-in .16s var(--ease); }
.confirm-text { font-size: 12px; margin-bottom: 8px; }
.confirm-actions { display: flex; justify-content: flex-end; gap: 6px; }
.confirm-actions .btn { padding: 6px 11px; font-size: 12px; }

/* Sub-tasks + sections */
.task { position: relative; }
.chevron { position: absolute; left: -22px; top: 10px; width: 20px; height: 20px; padding: 0;
  border: none; background: none; color: var(--muted); cursor: pointer; border-radius: var(--r-sm);
  display: flex; align-items: center; justify-content: center; transition: transform .18s var(--ease), background-color .16s; }
.chevron:hover { background: var(--icon-hover); color: var(--text); }
.chevron.collapsed { transform: rotate(-90deg); }
.section-block { margin-top: 18px; }
.section-row { position: relative; }
.section-actions { display: none; margin-left: auto; gap: 2px; }
.section-row:hover .section-actions { display: inline-flex; }
.section-rename { padding: 14px 0 6px; }
.add-section-row { display: flex; align-items: center; gap: 10px; margin-top: 18px; padding: 6px 10px;
  color: var(--accent); cursor: pointer; opacity: 0; transition: opacity .16s var(--ease); }
.add-section-row:hover { opacity: 1; }
.add-section-line { flex: 1; height: 1px; background: var(--accent); border-radius: 1px; }
.add-section-label { font-size: 13px; font-weight: 600; }

/* Filters */
.mono { font-family: ui-monospace, "Cascadia Code", Consolas, Menlo, monospace; font-size: 12px; }
.view-sub.mono { color: var(--subtle); }
.nav-icon.filter { color: var(--accent); }
.filter-form { display: flex; flex-direction: column; gap: 6px; padding: 6px 8px 10px; }
.project-input.invalid { border-color: var(--due-overdue); }
.filter-problem { color: var(--due-overdue); font-size: 12px; line-height: 16px; }
.filter-hint { color: var(--muted); font-size: 11px; line-height: 15px; }
.big-problem { font-size: 13px; padding: 16px 0; }

.empty { text-align: center; color: var(--muted); padding: 56px 0 24px; }
.empty .big { font-size: 15px; color: var(--text); font-weight: 600; margin-bottom: 4px; }
.error { color: var(--due-overdue); padding: 24px; }

/* Deadlines + task reminders */
.deadline { display: inline-flex; align-items: center; gap: 4px; }
.deadline.set { color: var(--due-week); }
.deadline.urgent { color: var(--due-overdue); font-weight: 500; }
.reminder-tag { display: inline-flex; align-items: center; color: var(--muted); }
.deadline-chip.set { color: var(--due-week); }
.deadline-chip.urgent { color: var(--due-overdue); }
.reminder-chip { color: var(--accent); }
.quick-chip.chip-deadline { color: var(--due-week); }
.hl-deadline { --hl-bg: rgba(105,47,194,.14); }
.reminder-pop { min-width: 250px; }
.reminder-item { display: flex; align-items: center; gap: 10px; padding: 4px 4px 4px 8px; font-size: 13px; }
.reminder-item .icon-btn { font-size: 16px; line-height: 1; width: 26px; height: 26px; }
.menu-divider { height: 1px; background: var(--divider); margin: 6px 2px; }
.menu-item:disabled { opacity: .4; cursor: default; background: none; }
.label-new.bad { border-color: var(--due-overdue); }

/* Search view */
.nav-icon.search { color: var(--subtle); }
.search-box { position: relative; display: flex; align-items: center; margin: 6px 0 14px; }
.search-icon { position: absolute; left: 12px; display: inline-flex; color: var(--muted); pointer-events: none; }
.search-input { width: 100%; height: 42px; padding: 0 14px 0 40px; border: 1px solid var(--border); border-radius: var(--r-md);
  font-size: 15px; outline: none; background: var(--bg); color: var(--text); transition: border-color .16s, box-shadow .16s; }
.search-input:focus { border-color: var(--border-strong); box-shadow: 0 0 0 3px var(--focus-ring); }
.search-input::placeholder { color: var(--muted); }

/* Settings window */
.modal-scrim { position: fixed; inset: 0; z-index: 60; background: rgba(0,0,0,.32); display: flex;
  align-items: flex-start; justify-content: center; padding: 10vh 12px 12px; animation: fade-in .18s var(--ease); }
.modal { position: relative; width: min(460px, 100%); max-height: 80vh; overflow-y: auto; outline: none;
  background: var(--bg); border-radius: var(--r-lg); box-shadow: var(--shadow-lg); padding: 6px 20px 16px;
  animation: pop-in .2s var(--ease); }
.modal .pop-backdrop { z-index: 64; }
.modal .popover { z-index: 70; }
.modal-head { display: flex; align-items: center; justify-content: space-between; padding: 10px 0 4px; }
.modal-head h2 { margin: 0; font-size: 17px; font-weight: 700; }
.modal-head .icon-btn { font-size: 20px; line-height: 1; width: 30px; height: 30px; }
.modal-section { margin: 14px 0 4px; font-size: 12px; font-weight: 600; color: var(--muted);
  text-transform: uppercase; letter-spacing: .04em; }
.setting-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 10px 0;
  border-bottom: 1px solid var(--divider); }
.setting-text { min-width: 0; }
.setting-name { font-size: 14px; }
.setting-hint { font-size: 12px; color: var(--muted); margin-top: 2px; }
.setting-warning { margin: 8px 0 4px; padding: 8px 10px; border-radius: var(--r); font-size: 12.5px;
  background: var(--p1-bg); color: var(--p1); }
.setting-group { transition: opacity .16s var(--ease); }
.setting-group.disabled { opacity: .45; pointer-events: none; }
.switch { position: relative; width: 40px; height: 24px; flex-shrink: 0; border: none; border-radius: 999px;
  background: var(--border-strong); cursor: pointer; padding: 0; transition: background-color .18s var(--ease); }
.switch .knob { position: absolute; top: 3px; left: 3px; width: 18px; height: 18px; border-radius: 50%;
  background: #fff; box-shadow: var(--shadow-sm); transition: transform .18s var(--ease); }
.switch.on { background: var(--accent); }
.switch.on .knob { transform: translateX(16px); }
.upcoming-list { list-style: none; margin: 6px 0 0; padding: 0; }
.upcoming-list li { display: flex; gap: 12px; padding: 6px 0; font-size: 13px; }
.upcoming-when { width: 110px; flex-shrink: 0; color: var(--due-today); font-variant-numeric: tabular-nums; }
.upcoming-title { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.upcoming-empty { padding: 8px 0; }
.file-btn { position: relative; overflow: hidden; display: inline-flex; align-items: center; flex-shrink: 0; }
.file-btn input { position: absolute; inset: 0; opacity: 0; cursor: pointer; }
.file-btn.busy { opacity: .6; }
.import-results { list-style: none; margin: 6px 0 0; padding: 0; font-size: 13px; }
.import-results li { display: flex; align-items: flex-start; gap: 8px; padding: 4px 0; }
.import-results li.ok .icon { color: var(--accent); }
.import-results li.bad { color: var(--due-overdue); }
.modal-foot { display: flex; gap: 8px; align-items: flex-start; margin: 14px 0 0; font-size: 12px;
  color: var(--muted); line-height: 17px; }
.modal-foot .icon { flex-shrink: 0; margin-top: 1px; }

/* Scrollbars: thin and rounded */
::-webkit-scrollbar { width: 10px; height: 10px; }
::-webkit-scrollbar-thumb { background: var(--border); border-radius: 999px; border: 3px solid transparent; background-clip: content-box; }
::-webkit-scrollbar-thumb:hover { background: var(--border-strong); background-clip: content-box; }
::-webkit-scrollbar-track { background: transparent; }

/* Narrow windows and phones: the sidebar becomes a slide-over drawer (opened
   by the header's hamburger) and the main pane goes full-bleed. */
@keyframes fade-in { from { opacity: 0; } to { opacity: 1; } }
@media (max-width: 700px) {
  .sidebar { position: fixed; top: 0; left: 0; bottom: 0; z-index: 50; width: min(360px, 88vw);
    background: var(--app-bg); box-shadow: var(--shadow-lg);
    padding-top: calc(14px + env(safe-area-inset-top)); padding-bottom: calc(14px + env(safe-area-inset-bottom));
    transform: translateX(-100%); visibility: hidden;
    transition: transform .26s var(--ease), visibility 0s linear .26s; }
  .app.drawer-open .sidebar { transform: none; visibility: visible; transition: transform .26s var(--ease); }
  .drawer-scrim { position: fixed; inset: 0; z-index: 40; background: rgba(0,0,0,.45); animation: fade-in .2s var(--ease); }
  /* Parallax: the page sits a little to the right while the drawer is open
     and glides back as it closes (`translate`, so it composes with the
     fade-in's animation). */
  .content { transition: translate .26s var(--ease); }
  .app.drawer-open .content { translate: 28px 0; }
  .menu-btn { display: inline-flex; }
  .main { margin: 0; border-radius: 0; box-shadow: none; }
  .content { padding: calc(10px + env(safe-area-inset-top)) 16px 80px; }
  .view-title { font-size: 22px; }
  .view-toggle { padding: 6px 8px; }
  .section-title { padding-left: 8px; padding-right: 8px; }
  /* Parent chevrons sit in the (now thin) gutter. */
  .chevron { left: -16px; width: 16px; }
  /* Row actions float over the row instead of reserving width for the title. */
  .task-row { gap: 10px; padding: 10px 8px 9px; }
  .task-actions { position: absolute; top: 4px; right: 4px; padding: 2px; border-radius: var(--r);
    background: var(--row-hover); box-shadow: var(--shadow-sm); pointer-events: none; }
  .task.pressed .task-actions, .task-actions:focus-within { pointer-events: auto; }
  .task-project { max-width: 38%; overflow: hidden; text-overflow: ellipsis; }
  .streak-panel { margin-left: 0; gap: 12px; }
  /* Editor: chips wrap, pickers stay on screen, the footer sticks to the bottom. */
  .editor { padding: 10px 10px 0; }
  .chips { position: relative; gap: 6px; }
  .chip { min-width: 0; max-width: 100%; }
  .chip input, .chip select { min-width: 0; max-width: 100%; }
  .chip-btn { max-width: 100%; }
  .chip-wrap { position: static; max-width: 100%; }
  .popover { max-width: calc(100vw - 16px); }
  .menu { min-width: min(260px, calc(100vw - 16px)); }
  .editor-footer { position: sticky; bottom: 0; z-index: 5; background: var(--bg);
    border-radius: 0 0 var(--r-lg) var(--r-lg); padding-bottom: calc(10px + env(safe-area-inset-bottom)); }
  .editor-footer .btn { flex-shrink: 0; }
}

@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after { animation: none !important; transition: none !important; }
}
"#;

/// The full stylesheet: light by default, dark when the OS prefers it (unless
/// forced light), or when forced dark.
pub fn css() -> String {
    format!(
        "{fonts}:root {{{LIGHT}}}\n\
         @media (prefers-color-scheme: dark) {{ :root:not([data-theme=\"light\"]) {{{DARK}}} }}\n\
         :root[data-theme=\"dark\"] {{{DARK}}}\n\
         {RULES}",
        fonts = fonts()
    )
}

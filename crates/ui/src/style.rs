//! One stylesheet mirroring Todoist's layout, spacing and palette, with light
//! and dark themes as CSS variables. (Own CSS; no Todoist assets.)
//!
//! Theme resolution: `data-theme="light|dark"` on <html> wins; with no
//! attribute ("System") the OS `prefers-color-scheme` decides.

const LIGHT: &str = r#"
  color-scheme: light;
  --bg: #ffffff;
  --sidebar: #fcfaf8;
  --text: #202020;
  --muted: #808080;
  --subtle: #666666;
  --divider: #f0f0f0;
  --hover: #f2efed;
  --icon-hover: #eeeeee;
  --active-bg: #ffefe5;
  --active-text: #b03d32;
  --border: #dddddd;
  --border-strong: #b8b8b8;
  --chip-border: #e5e5e5;
  --chip-bg: #ffffff;
  --chip-text: #444444;
  --panel: #fcfaf8;
  --btn-cancel: #f5f5f5;
  --btn-cancel-hover: #e8e8e8;
  --btn-cancel-text: #444444;
  --accent: #dc4c3e;
  --accent-hover: #c3392c;
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
  --bg: #1e1e1e;
  --sidebar: #282828;
  --text: #e6e6e6;
  --muted: #9a9a9a;
  --subtle: #b0b0b0;
  --divider: #333333;
  --hover: #363636;
  --icon-hover: #3d3d3d;
  --active-bg: #4a2a26;
  --active-text: #ff8a7f;
  --border: #444444;
  --border-strong: #6b6b6b;
  --chip-border: #444444;
  --chip-bg: #282828;
  --chip-text: #d0d0d0;
  --panel: #282828;
  --btn-cancel: #363636;
  --btn-cancel-hover: #444444;
  --btn-cancel-text: #e0e0e0;
  --accent: #dc4c3e;
  --accent-hover: #e86a5e;
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
* { box-sizing: border-box; }
html, body { margin: 0; height: 100%; }
body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  font-size: 14px; color: var(--text); background: var(--bg);
  -webkit-font-smoothing: antialiased;
}
button, input, select { font-family: inherit; color: inherit; }

.app { display: flex; height: 100vh; background: var(--bg); }

/* Sidebar */
.sidebar { width: 280px; flex-shrink: 0; background: var(--sidebar); padding: 12px 10px; overflow-y: auto; }
.brand { display: flex; align-items: center; gap: 8px; font-weight: 700; font-size: 15px; padding: 6px 8px 18px; }
.brand-mark { width: 26px; height: 26px; border-radius: 7px; background: var(--accent); color: #fff;
  display: flex; align-items: center; justify-content: center; font-size: 14px; }
.brand .theme-toggle { margin-left: auto; }
.nav-item { display: flex; align-items: center; gap: 8px; height: 34px; padding: 0 8px; border-radius: 5px;
  cursor: pointer; color: var(--text); user-select: none; }
.nav-item:hover { background: var(--hover); }
.nav-item.active { background: var(--active-bg); color: var(--active-text); }
.nav-item .count { margin-left: auto; color: var(--muted); font-size: 12px; }
.nav-item.active .count { color: var(--active-text); }
.nav-icon { width: 24px; height: 24px; display: flex; align-items: center; justify-content: center; }
.nav-icon.inbox { color: var(--inbox); }
.nav-icon.today { color: var(--today); }
.nav-icon.upcoming { color: var(--upcoming); }
.nav-icon.hash { color: var(--muted); font-size: 16px; font-weight: 500; }
.section-head { display: flex; align-items: center; justify-content: space-between;
  padding: 18px 8px 4px; font-weight: 700; color: var(--subtle); font-size: 13px; }
.project-form { padding: 4px 8px; }
.project-input { width: 100%; padding: 6px 8px; border: 1px solid var(--border); border-radius: 5px;
  font-size: 13px; outline: none; background: var(--bg); }
.project-input:focus { border-color: var(--border-strong); }

.icon { display: inline-flex; align-items: center; justify-content: center; }
.icon-btn { border: none; background: none; cursor: pointer; color: var(--muted); padding: 4px; border-radius: 4px;
  display: inline-flex; align-items: center; justify-content: center; }
.icon-btn:hover { background: var(--icon-hover); color: var(--text); }

/* Main pane */
.main { flex: 1; overflow-y: auto; background: var(--bg); }
.content { max-width: 820px; margin: 0 auto; padding: 36px 55px 96px; }
.view-title { font-size: 26px; font-weight: 700; margin: 0 0 2px; }
.view-sub { color: var(--muted); font-size: 12px; margin: 0 0 12px; }
.section-title { display: flex; align-items: baseline; gap: 6px; font-weight: 700; font-size: 14px;
  padding: 20px 0 8px; border-bottom: 1px solid var(--divider); }
.section-title .muted { color: var(--muted); font-weight: 400; font-size: 13px; }

/* Task rows */
.task { border-bottom: 1px solid var(--divider); }
.task-row { display: flex; align-items: flex-start; gap: 10px; padding: 10px 0 8px; }
.check { width: 18px; height: 18px; min-width: 18px; margin-top: 1px; border-radius: 50%; padding: 0;
  border: 1px solid var(--p4); background: transparent; color: var(--muted); cursor: pointer;
  display: flex; align-items: center; justify-content: center; }
.check .icon { opacity: 0; transition: opacity .12s; }
.check:hover .icon { opacity: 1; }
.check.p1 { border: 2px solid var(--p1); background: var(--p1-bg); color: var(--p1); }
.check.p2 { border: 2px solid var(--p2); background: var(--p2-bg); color: var(--p2); }
.check.p3 { border: 2px solid var(--p3); background: var(--p3-bg); color: var(--p3); }
.task-body { flex: 1; min-width: 0; }
.task-content { line-height: 20px; word-wrap: break-word; }
.task-desc { color: var(--muted); font-size: 12px; line-height: 16px; margin-top: 1px; }
.task-meta { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; margin-top: 4px; font-size: 12px; }
.due { display: inline-flex; align-items: center; gap: 3px; }
.due.overdue { color: var(--due-overdue); }
.due.today { color: var(--due-today); }
.due.tomorrow { color: var(--due-tomorrow); }
.due.week { color: var(--due-week); }
.due.later { color: var(--due-later); }
.streak { color: var(--streak); font-weight: 500; }
.task-project { font-size: 12px; color: var(--muted); white-space: nowrap; align-self: flex-end; padding-bottom: 2px; }
.task-actions { display: flex; gap: 2px; opacity: 0; transition: opacity .12s; }
.task:hover .task-actions { opacity: 1; }
.streak-panel { display: flex; flex-wrap: wrap; align-items: center; gap: 18px; margin: 0 0 10px 28px;
  padding: 10px 12px; background: var(--panel); border: 1px solid var(--divider); border-radius: 8px; font-size: 13px; }
.streak-panel .stat b { font-size: 15px; }
.streak-panel input { width: 64px; padding: 4px 6px; border: 1px solid var(--border); border-radius: 4px; background: var(--bg); }

/* Add task */
.add-task-row { display: flex; align-items: center; gap: 10px; padding: 8px 0; color: var(--muted); cursor: pointer; }
.add-task-row .plus { width: 18px; height: 18px; border-radius: 50%; color: var(--accent);
  display: flex; align-items: center; justify-content: center; }
.add-task-row:hover { color: var(--accent); }
.add-task-row:hover .plus { background: var(--accent); color: #fff; }
.editor { border: 1px solid var(--border); border-radius: 10px; padding: 10px 10px 0; margin: 8px 0; background: var(--bg); }
.editor:focus-within { border-color: var(--border-strong); }
.editor .name { width: 100%; border: none; outline: none; font-size: 14px; font-weight: 600; padding: 2px 0; background: transparent; }
.editor .desc { width: 100%; border: none; outline: none; font-size: 13px; padding: 4px 0 10px; background: transparent; }
.editor .name::placeholder, .editor .desc::placeholder, .project-input::placeholder { color: var(--muted); }
.chips { display: flex; flex-wrap: wrap; gap: 8px; padding-bottom: 10px; }
.chip { display: inline-flex; align-items: center; gap: 4px; height: 28px; padding: 0 6px;
  border: 1px solid var(--chip-border); border-radius: 5px; font-size: 12px; color: var(--subtle); background: var(--chip-bg); }
.chip input, .chip select { border: none; outline: none; background: transparent; font-size: 12px; color: var(--chip-text); }
.chip select option { background: var(--chip-bg); color: var(--chip-text); }
.editor-footer { display: flex; justify-content: flex-end; gap: 8px; border-top: 1px solid var(--divider); padding: 10px 0; }
.btn { border: none; border-radius: 5px; padding: 7px 12px; font-size: 13px; font-weight: 600; cursor: pointer; }
.btn-cancel { background: var(--btn-cancel); color: var(--btn-cancel-text); }
.btn-cancel:hover { background: var(--btn-cancel-hover); }
.btn-add { background: var(--accent); color: #fff; }
.btn-add:hover { background: var(--accent-hover); }
.btn-add:disabled { opacity: .45; cursor: not-allowed; }

/* Labels + editor pickers */
.task-body { cursor: pointer; }
.label-tag { display: inline-flex; align-items: center; gap: 3px; color: var(--muted); }
.nav-icon.label { color: var(--muted); }
.chip-hash { color: var(--muted); font-weight: 600; }
.chip-btn { cursor: pointer; font-family: inherit; max-width: 220px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.chip-wrap { position: relative; }
.label-pop { position: absolute; top: 32px; left: 0; z-index: 20; min-width: 220px; padding: 6px;
  background: var(--bg); border: 1px solid var(--border); border-radius: 8px;
  box-shadow: 0 6px 20px rgba(0,0,0,.18); }
.label-opt { display: flex; align-items: center; gap: 8px; padding: 5px 6px; border-radius: 4px; cursor: pointer; font-size: 13px; }
.label-opt:hover { background: var(--hover); }
.label-empty { color: var(--muted); font-size: 12px; padding: 4px 6px; }
.label-new { width: 100%; margin-top: 6px; padding: 5px 6px; font-size: 12px; border: 1px solid var(--border);
  border-radius: 4px; outline: none; background: var(--bg); }

/* Completed tasks + sidebar management */
.view-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.view-toggle { display: inline-flex; align-items: center; gap: 6px; border: none; background: none; cursor: pointer;
  color: var(--muted); font-size: 13px; padding: 4px 8px; border-radius: 5px; }
.view-toggle:hover { background: var(--icon-hover); color: var(--text); }
.check.done { background: var(--muted); border: 1px solid var(--muted); color: var(--bg); }
.check.done .icon { opacity: 1; }
.task.completed .task-content { text-decoration: line-through; color: var(--muted); }
.task.completed .task-body { cursor: default; }
.done-when { color: var(--muted); }
.nav-name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.nav-actions { display: none; gap: 0; }
.nav-item:hover .nav-actions { display: inline-flex; }
.nav-actions .icon-btn { padding: 2px; }
.confirm-box { margin: 4px 0; padding: 8px 10px; border: 1px solid var(--border); border-radius: 6px; background: var(--bg); }
.confirm-text { font-size: 12px; margin-bottom: 8px; }
.confirm-actions { display: flex; justify-content: flex-end; gap: 6px; }
.confirm-actions .btn { padding: 5px 10px; font-size: 12px; }

/* Sub-tasks + sections */
.task { position: relative; }
.chevron { position: absolute; left: -24px; top: 10px; width: 20px; height: 20px; padding: 0;
  border: none; background: none; color: var(--muted); cursor: pointer; border-radius: 4px;
  display: flex; align-items: center; justify-content: center; transition: transform .12s; }
.chevron:hover { background: var(--icon-hover); color: var(--text); }
.chevron.collapsed { transform: rotate(-90deg); }
.section-block { margin-top: 18px; }
.section-row { position: relative; }
.section-actions { display: none; margin-left: auto; gap: 2px; }
.section-row:hover .section-actions { display: inline-flex; }
.section-rename { padding: 14px 0 6px; }
.add-section-row { display: flex; align-items: center; gap: 10px; margin-top: 18px; padding: 6px 0;
  color: var(--accent); cursor: pointer; opacity: .55; transition: opacity .12s; }
.add-section-row:hover { opacity: 1; }
.add-section-line { flex: 1; height: 1px; background: var(--accent); }
.add-section-label { font-size: 13px; font-weight: 600; }

.empty { text-align: center; color: var(--muted); padding: 56px 0 24px; }
.empty .big { font-size: 15px; color: var(--text); font-weight: 600; margin-bottom: 4px; }
.error { color: var(--due-overdue); padding: 24px; }
"#;

/// The full stylesheet: light by default, dark when the OS prefers it (unless
/// forced light), or when forced dark.
pub fn css() -> String {
    format!(
        ":root {{{LIGHT}}}\n\
         @media (prefers-color-scheme: dark) {{ :root:not([data-theme=\"light\"]) {{{DARK}}} }}\n\
         :root[data-theme=\"dark\"] {{{DARK}}}\n\
         {RULES}"
    )
}

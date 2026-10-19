//! One stylesheet mirroring Todoist's layout, spacing and palette.
//! (Own CSS; no Todoist assets.)

pub const CSS: &str = r#"
* { box-sizing: border-box; }
html, body { margin: 0; height: 100%; }
body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  font-size: 14px; color: #202020; background: #fff;
  -webkit-font-smoothing: antialiased;
}
button, input, select { font-family: inherit; }

.app { display: flex; height: 100vh; }

/* Sidebar */
.sidebar { width: 280px; flex-shrink: 0; background: #fcfaf8; padding: 12px 10px; overflow-y: auto; }
.brand { display: flex; align-items: center; gap: 8px; font-weight: 700; font-size: 15px; padding: 6px 8px 18px; }
.brand-mark { width: 26px; height: 26px; border-radius: 7px; background: #dc4c3e; color: #fff;
  display: flex; align-items: center; justify-content: center; font-size: 14px; }
.nav-item { display: flex; align-items: center; gap: 8px; height: 34px; padding: 0 8px; border-radius: 5px;
  cursor: pointer; color: #202020; user-select: none; }
.nav-item:hover { background: #f2efed; }
.nav-item.active { background: #ffefe5; color: #b03d32; }
.nav-item .count { margin-left: auto; color: #808080; font-size: 12px; }
.nav-item.active .count { color: #b03d32; }
.nav-icon { width: 24px; height: 24px; display: flex; align-items: center; justify-content: center; }
.nav-icon.inbox { color: #246fe0; }
.nav-icon.today { color: #058527; }
.nav-icon.upcoming { color: #692fc2; }
.nav-icon.hash { color: #808080; font-size: 16px; font-weight: 500; }
.section-head { display: flex; align-items: center; justify-content: space-between;
  padding: 18px 8px 4px; font-weight: 700; color: #666; font-size: 13px; }
.project-form { padding: 4px 8px; }
.project-input { width: 100%; padding: 6px 8px; border: 1px solid #ddd; border-radius: 5px; font-size: 13px; outline: none; }
.project-input:focus { border-color: #999; }

.icon { display: inline-flex; align-items: center; justify-content: center; }
.icon-btn { border: none; background: none; cursor: pointer; color: #808080; padding: 4px; border-radius: 4px;
  display: inline-flex; align-items: center; justify-content: center; }
.icon-btn:hover { background: #eeeeee; color: #202020; }

/* Main pane */
.main { flex: 1; overflow-y: auto; }
.content { max-width: 820px; margin: 0 auto; padding: 36px 55px 96px; }
.view-title { font-size: 26px; font-weight: 700; margin: 0 0 2px; }
.view-sub { color: #808080; font-size: 12px; margin: 0 0 12px; }
.section-title { display: flex; align-items: baseline; gap: 6px; font-weight: 700; font-size: 14px;
  padding: 20px 0 8px; border-bottom: 1px solid #f0f0f0; }
.section-title .muted { color: #808080; font-weight: 400; font-size: 13px; }
.section-title.overdue { color: #202020; }

/* Task rows */
.task { border-bottom: 1px solid #f0f0f0; }
.task-row { display: flex; align-items: flex-start; gap: 10px; padding: 10px 0 8px; }
.check { width: 18px; height: 18px; min-width: 18px; margin-top: 1px; border-radius: 50%; padding: 0;
  border: 1px solid #b8b8b8; background: transparent; color: #808080; cursor: pointer;
  display: flex; align-items: center; justify-content: center; }
.check .icon { opacity: 0; transition: opacity .12s; }
.check:hover .icon { opacity: 1; }
.check.p1 { border: 2px solid #d1453b; background: rgba(209,69,59,.1); color: #d1453b; }
.check.p2 { border: 2px solid #eb8909; background: rgba(235,137,9,.1); color: #eb8909; }
.check.p3 { border: 2px solid #246fe0; background: rgba(36,111,224,.1); color: #246fe0; }
.task-body { flex: 1; min-width: 0; }
.task-content { line-height: 20px; word-wrap: break-word; }
.task-desc { color: #808080; font-size: 12px; line-height: 16px; margin-top: 1px; }
.task-meta { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; margin-top: 4px; font-size: 12px; }
.due { display: inline-flex; align-items: center; gap: 3px; }
.due.overdue { color: #d1453b; }
.due.today { color: #058527; }
.due.tomorrow { color: #ad6200; }
.due.week { color: #692fc2; }
.due.later { color: #808080; }
.streak { color: #c2410c; font-weight: 500; }
.task-project { font-size: 12px; color: #808080; white-space: nowrap; align-self: flex-end; padding-bottom: 2px; }
.task-actions { display: flex; gap: 2px; opacity: 0; transition: opacity .12s; }
.task:hover .task-actions { opacity: 1; }
.streak-panel { display: flex; flex-wrap: wrap; align-items: center; gap: 18px; margin: 0 0 10px 28px;
  padding: 10px 12px; background: #fcfaf8; border: 1px solid #f0f0f0; border-radius: 8px; font-size: 13px; }
.streak-panel .stat b { font-size: 15px; }
.streak-panel input { width: 64px; padding: 4px 6px; border: 1px solid #ddd; border-radius: 4px; }

/* Add task */
.add-task-row { display: flex; align-items: center; gap: 10px; padding: 8px 0; color: #808080; cursor: pointer; }
.add-task-row .plus { width: 18px; height: 18px; border-radius: 50%; color: #dc4c3e;
  display: flex; align-items: center; justify-content: center; }
.add-task-row:hover { color: #dc4c3e; }
.add-task-row:hover .plus { background: #dc4c3e; color: #fff; }
.editor { border: 1px solid #ddd; border-radius: 10px; padding: 10px 10px 0; margin: 8px 0; }
.editor:focus-within { border-color: #b8b8b8; }
.editor .name { width: 100%; border: none; outline: none; font-size: 14px; font-weight: 600; padding: 2px 0; }
.editor .desc { width: 100%; border: none; outline: none; font-size: 13px; padding: 4px 0 10px; }
.chips { display: flex; flex-wrap: wrap; gap: 8px; padding-bottom: 10px; }
.chip { display: inline-flex; align-items: center; gap: 4px; height: 28px; padding: 0 6px;
  border: 1px solid #e5e5e5; border-radius: 5px; font-size: 12px; color: #666; background: #fff; }
.chip input, .chip select { border: none; outline: none; background: transparent; font-size: 12px; color: #444; }
.editor-footer { display: flex; justify-content: flex-end; gap: 8px; border-top: 1px solid #f0f0f0; padding: 10px 0; }
.btn { border: none; border-radius: 5px; padding: 7px 12px; font-size: 13px; font-weight: 600; cursor: pointer; }
.btn-cancel { background: #f5f5f5; color: #444; }
.btn-cancel:hover { background: #e8e8e8; }
.btn-add { background: #dc4c3e; color: #fff; }
.btn-add:hover { background: #c3392c; }
.btn-add:disabled { opacity: .45; cursor: not-allowed; }

.empty { text-align: center; color: #808080; padding: 56px 0 24px; }
.empty .big { font-size: 15px; color: #202020; font-weight: 600; margin-bottom: 4px; }
.error { color: #d1453b; padding: 24px; }
"#;

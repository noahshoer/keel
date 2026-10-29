// Sub-task counts and streaks sit at the right of the title's line, with
// the project under them.
const rows = [...document.querySelectorAll('.task-row')].slice(0, 6);
for (const row of rows) {
  const r = row.getBoundingClientRect();
  const name = row.querySelector('.task-content')?.textContent;
  const badge = row.querySelector('.task-badges')?.getBoundingClientRect();
  const title = row.querySelector('.task-content').getBoundingClientRect();
  const proj = row.querySelector('.task-project')?.getBoundingClientRect();
  const kids = [...(row.querySelector('.task-badges')?.children ?? [])].map((c) => (c.textContent.trim() ? c.textContent.trim() : c.className)).join(' + ');
  check(`${name}: badges [${kids}] right gap ${badge ? Math.round(r.right - badge.right) : '-'}, same line as title ${badge && kids ? Math.abs(badge.top - title.top) < 4 : '-'}, project right gap ${proj ? Math.round(r.right - proj.right) : '-'}`);
}

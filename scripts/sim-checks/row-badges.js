// Rows run title and streak, deadline, description, then the bottom rows: the
// when group never shortens, and the project stays pinned right on one line.
const rows = [...document.querySelectorAll('.task-row')].slice(0, 8);
for (const row of rows) {
  const r = row.getBoundingClientRect();
  const name = row.querySelector('.task-content')?.textContent;
  const when = row.querySelector('.task-when');
  const proj = row.querySelector('.task-project')?.getBoundingClientRect();
  const bottom = row.querySelector('.task-bottom')?.getBoundingClientRect();
  const parts = [...(when?.children ?? [])].map((c) => c.className.split(' ')[0]).join(' + ');
  const clipped = when ? when.scrollWidth > when.clientWidth + 1 : false;
  check(`${name}: when [${parts}] clipped ${clipped}, labels ${!!row.querySelector('.task-labels')}, project "${row.querySelector('.task-project')?.textContent}" right gap ${proj ? Math.round(r.right - proj.right) : '-'}, one line ${proj && bottom ? proj.height < 30 : '-'}`);
}

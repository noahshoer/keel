// Quick add from Today's "+ Add task": the typed phrases are highlighted
// and previewed, Enter adds the task, every picker fits the screen, and the
// new row shows up. Adds a task named "Quick add check" to the simulator's
// data.
const vw = innerWidth, vh = innerHeight;
const today = [...document.querySelectorAll('.nav-item')].find((n) => n.textContent.includes('Today'));
today?.click();
await sleep(800);
const row = document.querySelector('.add-task-row.view-add');
check(`add row: ${!!row}`);
row.click();
await sleep(300);
const title = document.querySelector('.title-input');
check(`title focused: ${document.activeElement === title}`);
const setter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value').set;
setter.call(title, 'Quick add check tomorrow 5pm p1');
title.dispatchEvent(new Event('input', { bubbles: true }));
await sleep(300);
check(`marks: ${[...document.querySelectorAll('.title-mirror mark')].map((m) => `${m.className}=${m.textContent}`).join(', ')}`);
check(`preview: ${[...document.querySelectorAll('.quick-chip')].map((c) => c.textContent).join(', ')}`);
for (const chip of document.querySelectorAll('.editor .chips .chip-btn')) {
  chip.click();
  await sleep(250);
  const pop = document.querySelector('.popover');
  const r = pop?.getBoundingClientRect();
  const inside = r && r.left >= 0 && r.top >= 0 && r.right <= vw && r.bottom <= vh;
  check(`${chip.title}: popover ${r ? `${Math.round(r.left)},${Math.round(r.top)} ${Math.round(r.width)}x${Math.round(r.height)}` : 'none'} inside ${inside}`);
  chip.click();
  await sleep(150);
}
title.focus();
title.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
await sleep(1200);
check(`title cleared: ${document.querySelector('.title-input')?.value === ''}`);
document.querySelector('.editor .btn-cancel')?.click();
const upcoming = [...document.querySelectorAll('.nav-item')].find((n) => n.textContent.includes('Upcoming'));
upcoming?.click();
await sleep(1000);
const added = [...document.querySelectorAll('.task-row')].find((r) => r.textContent.includes('Quick add check'));
check(`added row: ${added ? added.querySelector('.task-meta')?.textContent : 'missing'}`);

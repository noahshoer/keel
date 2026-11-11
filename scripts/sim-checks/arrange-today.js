// Arranging Today (issue #7): saves Noah's example (headings project > section
// > label, order by date then priority) through the real backend, reopens the
// page and checks the headings and sub-headings show; then resets it and checks
// they are gone. Leaves the data as it found it (the arrangement is reset, and
// the scenario stops first if Today already carried one).
const q = (s) => document.querySelector(s);
const place = { kind: 'today' };
const send = (arrangement) =>
  window.__TAURI__.core.invoke('dispatch', {
    args: { cmd: { uuid: crypto.randomUUID(), temp_id: null, action: { type: 'view_arrange', place, arrangement } } },
  });
// The page loads a page's arrangement with the view: leave Today and come back.
const navRow = (name) =>
  [...document.querySelectorAll('.sidebar .nav-card > div')].find((el) => el.textContent.includes(name));
const goto = async (name) => {
  // On the phone the rows sit in the drawer: open it only when it is closed.
  if (q('.menu-btn') && !q('.app.drawer-open')) { q('.menu-btn').click(); await sleep(1000); }
  navRow(name)?.click();
  await sleep(1500);
  if (q('.app.drawer-open') && q('.drawer-scrim')) { q('.drawer-scrim').click(); await sleep(800); }
};
const reopen = async () => { await goto('Inbox'); await goto('Today'); };
const state = () => ({
  title: [...document.querySelectorAll('.section-title')].some((el) => el.textContent.includes(' › ')),
  sub: !!q('.arrange-sub'),
});

await goto('Today');
const before = state();
check(`before: heading with " › " ${before.title}, sub-heading ${before.sub}`);
if (before.title || before.sub) { check('SKIPPED Today already arranged; nothing changed'); return; }
const was = await window.__TAURI__.core.invoke('list_arrangements', { args: {} });
check(`saved arrangements before: ${was.length}`);

await send({ filter: '', groups: ['project', 'section', 'label'], order: [{ field: 'due_date', reversed: false }, { field: 'priority', reversed: false }] });
await reopen();
const set = state();
check(`set: heading with " › " ${set.title} ${set.title ? 'PASS' : 'FAIL'}`);
check(`set: .arrange-sub ${set.sub ? 'PASS' : 'FAIL'}`);
await sleep(6000); // time for a screenshot of the arranged page

await send({ filter: '', groups: [], order: [] });
await reopen();
const reset = state();
check(`reset: heading with " › " ${reset.title ? 'FAIL' : 'PASS'}`);
check(`reset: .arrange-sub ${reset.sub ? 'FAIL' : 'PASS'}`);
const after = await window.__TAURI__.core.invoke('list_arrangements', { args: {} });
check(`saved arrangements after: ${after.length} ${after.length === was.length ? 'PASS' : 'FAIL'}`);

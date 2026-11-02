// Saving a new name, from a sidebar row and from a page: frame by frame, the
// sidebar's row goes straight to the new name, never back through the old one.
// Renames the first project and label, and back.
const setValue = (input, to) => {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, to);
  input.dispatchEvent(new Event('input', { bubbles: true }));
};
const watching = async (id, input, label) => {
  const row = () => document.querySelector(`.sidebar [data-menu-id="${id}"] .nav-name`);
  const frames = [];
  let on = true;
  const watch = () => {
    if (!on) return;
    const s = row()?.textContent ?? `field "${document.querySelector('.sidebar .project-form input')?.value}"`;
    if (frames[frames.length - 1]?.[0] !== s) frames.push([s, Math.round(performance.now())]);
    requestAnimationFrame(watch);
  };
  watch();
  input.form.requestSubmit();
  await sleep(800);
  on = false;
  check(`${label}:\n` + frames.map(([s, t]) => `  +${t - frames[0][1]}ms ${s}`).join('\n'));
};
const fromRow = async (id, to) => {
  const row = document.querySelector(`.sidebar [data-menu-id="${id}"]`);
  row.dispatchEvent(new Event('keel-row-press'));
  await sleep(200);
  [...document.querySelectorAll('.row-menu .menu-item')].find((b) => b.textContent === 'Edit').click();
  await sleep(300);
  const input = document.querySelector('.sidebar .project-form input');
  setValue(input, to);
  await sleep(100);
  await watching(id, input, `row of ${id} to "${to}"`);
};
const fromPage = async (id, to) => {
  document.querySelector('.view-more').click();
  await sleep(200);
  [...document.querySelectorAll('.row-menu .menu-item')].find((b) => b.textContent === 'Edit').click();
  await sleep(300);
  const input = document.querySelector('.view-head input');
  setValue(input, to);
  await sleep(100);
  await watching(id, input, `page of ${id} to "${to}"`);
};
document.querySelector('.menu-btn').click();
await sleep(500);
for (const kind of ['project', 'label']) {
  const row = document.querySelector(`.sidebar [data-menu-id^="${kind}:"]`);
  const id = row.dataset.menuId;
  const name = row.querySelector('.nav-name').textContent;
  await fromRow(id, `${name}Z`);
  await fromRow(id, name);
}
const open = document.querySelector('.sidebar [data-menu-id^="project:"]');
const id = open.dataset.menuId;
const name = open.querySelector('.nav-name').textContent;
open.click();
await sleep(900);
await fromPage(id, `${name}Z`);
await fromPage(id, name);

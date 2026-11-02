// Saving a new title on a page: frame by frame, what's on screen goes straight
// from the field to the new name, never back through the old one. Renames the
// first project and back.
document.querySelector('.menu-btn').click();
await sleep(500);
document.querySelector('.sidebar .nav-item[data-menu-id^="project:"]').click();
await sleep(900);
const shown = () => {
  const h = document.querySelector('h1.view-title');
  const i = document.querySelector('.view-head input');
  const c = document.querySelector('.content');
  return `${i ? `field "${i.value}"` : `title "${h.textContent}"`} content ${c.className}`;
};
const rename = async (to) => {
  document.querySelector('.view-more').click();
  await sleep(200);
  [...document.querySelectorAll('.row-menu .menu-item')].find((b) => b.textContent === 'Edit').click();
  await sleep(300);
  const input = document.querySelector('.view-head input');
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, to);
  input.dispatchEvent(new Event('input', { bubbles: true }));
  await sleep(100);
  const frames = [];
  let on = true;
  const watch = () => { if (!on) return; const s = shown(); if (frames[frames.length - 1]?.[0] !== s) frames.push([s, Math.round(performance.now())]); requestAnimationFrame(watch); };
  watch();
  input.form.requestSubmit();
  await sleep(700);
  on = false;
  const start = frames[0][1];
  check(`to "${to}":\n` + frames.map(([s, t]) => `  +${t - start}ms ${s}`).join('\n'));
};
const name = document.querySelector('h1.view-title').textContent;
await rename(`${name}Z`);
await rename(name);

// On touch a tracked streak opens on the keyboard with its goal focused, and the
// goal shows in the streak row once set. A headless simulator has no keyboard,
// so this reports one to the page as Swift does (`window.keelKeyboard`).
// Expect the card's bottom 8 px above the keyboard's top, rising and landed.
const tap = (list, re) => [...document.querySelectorAll(list)].find((b) => re.test(b.innerText))?.click();
const card = () => document.querySelector('.streak-pop');
document.querySelector('.task-body').click();
await sleep(1500);
if (!document.querySelector('.tv-streak')) {
  if (!/Every|every/.test(document.querySelector('.tv-sheet').innerText)) {
    tap('.tv-pill', /Repeat/);
    await sleep(500);
    tap('.popover button', /^Every day$/);
    await sleep(800);
  }
  document.querySelector('.tv-more').click();
  await sleep(400);
  tap('.menu-item', /Streak/);
  await sleep(600);
  tap('.streak-panel button', /Track streak/);
  await sleep(800);
  document.querySelector('.pop-backdrop').click();
  await sleep(500);
}
const report = (when) => {
  const c = card().getBoundingClientRect();
  const s = document.querySelector('.tv-sheet').getBoundingClientRect();
  check(`${when}: card ${Math.round(c.top)}-${Math.round(c.bottom)}, sheet ends ${Math.round(s.bottom)}, width ${Math.round(c.width)}/${innerWidth}`);
};
const row = () => [...document.querySelectorAll('.tv-streak-stat')].map((s) => `${s.title} ${s.textContent}`).join(', ');
check(`row: ${row()}`);
document.querySelector('.tv-streak').click();
check(`focused the goal in the tap: ${document.activeElement === card().querySelector('input')}`);
report('opened');
const kb = 336;
window.keelKeyboard({ kb, secs: 0.25, ease: 'ease' });
// Every frame of the rise: the card's bottom should only climb.
const frames = [];
const from = performance.now();
await new Promise((done) => {
  const frame = () => {
    frames.push(Math.round(card().getBoundingClientRect().bottom));
    if (performance.now() - from < 500) requestAnimationFrame(frame);
    else done();
  };
  requestAnimationFrame(frame);
});
check(`rising, by frame: ${frames.filter((b, i) => b !== frames[i - 1]).join(' ')}`);
await sleep(400);
report(`landed, keyboard top ${document.documentElement.clientHeight - kb}`);
await sleep(1500);
const input = card().querySelector('input');
Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, '30');
input.dispatchEvent(new Event('input', { bubbles: true }));
await sleep(300);
document.querySelector('.pop-backdrop').click();
await sleep(1200);
check(`closed: ${card() === null}, row: ${row()}`);

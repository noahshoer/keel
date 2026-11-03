// The streak goal stays in view above the keyboard. A headless simulator has no
// keyboard, so this reports one to the page as Swift does (`window.keelKeyboard`).
// Expect the goal's bottom to end above the sheet's scroll bottom, and that
// bottom at the keyboard's top.
const sheetOf = () => document.querySelector('.tv-sheet .task-view:not(.tv-under) .tv-scroll');
const tap = (list, re) => [...document.querySelectorAll(list)].find((b) => re.test(b.innerText))?.click();
document.querySelector('.task-body').click();
await sleep(1500);
if (!document.querySelector('.tv-field-btn.tv-streak')) {
  tap('.tv-pill', /Repeat/);
  await sleep(500);
  tap('.popover button', /^Every day$/);
  await sleep(800);
}
const more = document.querySelector('.tv-head-actions button:last-child');
more.click();
await sleep(400);
tap('.menu-item', /Streak/);
await sleep(600);
tap('.streak-panel button', /Track streak/);
await sleep(800);
const input = document.querySelector('.streak-panel input');
const report = (when) => {
  const s = sheetOf().getBoundingClientRect();
  const g = input.getBoundingClientRect();
  check(`${when}: goal ${Math.round(g.top)}-${Math.round(g.bottom)}, scroll ends ${Math.round(s.bottom)}, screen ${document.documentElement.clientHeight}, scrollTop ${Math.round(sheetOf().scrollTop)}`);
};
// A task with many fields: the goal starts low enough for the rising sheet to cover it.
sheetOf().firstElementChild.style.marginTop = '400px';
sheetOf().scrollTop = 0;
await sleep(300);
sheetOf().scrollTop = input.getBoundingClientRect().bottom - sheetOf().getBoundingClientRect().bottom + 20;
await sleep(300);
report('before');
input.focus();
const kb = 336;
window.keelKeyboard({ kb, secs: 0.25, ease: 'ease' });
await sleep(100);
report('rising');
await sleep(1400);
report(`keyboard ${kb}`);
check(`active: ${document.activeElement === input}, top of keyboard ${document.documentElement.clientHeight - kb}`);

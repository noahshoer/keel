// The streak opens in a popover from the streak row and from the •••, on screen
// and under what opened it. A goal typed there saves when it closes. A headless
// simulator has no keyboard: sitting above one is for the phone.
const tap = (list, re) => [...document.querySelectorAll(list)].find((b) => re.test(b.innerText))?.click();
const box = (el) => {
  const r = el.getBoundingClientRect();
  return `${Math.round(r.left)},${Math.round(r.top)} to ${Math.round(r.right)},${Math.round(r.bottom)}`;
};
const pop = () => document.querySelector('.streak-pop');
document.querySelector('.task-body').click();
await sleep(1500);
if (!document.querySelector('.tv-field-btn.tv-streak')) {
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
  check(`from the more button ${box(document.querySelector('.tv-more'))}: popover ${box(pop())}, says ${pop().innerText.replace(/\n/g, ' / ')}`);
  tap('.streak-panel button', /Track streak/);
  await sleep(800);
  document.querySelector('.pop-backdrop').click();
  await sleep(500);
}
document.querySelector('.tv-streak').click();
await sleep(600);
check(`from the row ${box(document.querySelector('.tv-streak'))}: popover ${box(pop())}, screen ${innerWidth}x${innerHeight}`);
const input = pop().querySelector('input');
input.focus();
// React tracks `.value`; the prototype's setter is what typing looks like to it.
Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, '30');
input.dispatchEvent(new Event('input', { bubbles: true }));
await sleep(500);
document.querySelector('.pop-backdrop').click();
await sleep(1200);
check(`closed: ${pop() === null}`);
document.querySelector('.tv-streak').click();
await sleep(800);
check(`goal after reopening: ${pop().querySelector('input').value}`);

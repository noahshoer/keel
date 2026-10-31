// Open a task, then a sub-task, and swipe in from the left edge: a short
// swipe springs back, a long one goes back to the parent.
const touch = (type, x, y) => {
  const el = document.elementFromPoint(x, y) ?? document.body;
  const t = new Touch({ identifier: 1, target: el, clientX: x, clientY: y });
  el.dispatchEvent(new TouchEvent(type, { touches: type === 'touchend' ? [] : [t], changedTouches: [t], bubbles: true, cancelable: true }));
};
const swipe = async (dx) => {
  const y = 400;
  touch('touchstart', 6, y);
  for (let i = 1; i <= 8; i++) { await sleep(16); touch('touchmove', 6 + (dx * i) / 8, y); }
  touch('touchend', 6 + dx, y);
};
const title = () => document.querySelector('.tv-sheet .task-view:not(.tv-under) .tv-title textarea')?.value ?? '?';
document.querySelector('.task-body').click();
await sleep(3000);
document.querySelector('.tv-sub-body').click();
await sleep(2500);
check(`in sub-task: ${title()} back=${!!document.querySelector('.tv-back')}`);
await swipe(60);
await sleep(800);
check(`after short swipe: ${title()} back=${!!document.querySelector('.tv-back')}`);
// Held half way: the parent shows under the sliding content.
touch('touchstart', 6, 400);
for (let i = 1; i <= 8; i++) { await sleep(16); touch('touchmove', 6 + (120 * i) / 8, 400); }
const pk = document.querySelector('.tv-under');
check(`mid-drag: under ${pk && getComputedStyle(pk).visibility} "${pk?.querySelector('.tv-title textarea')?.value}" at ${pk?.querySelector('.tv-body')?.style.transform}; top moved ${document.querySelector('.task-view:not(.tv-under) > .tv-body')?.style.transform}`);
await sleep(2500);
touch('touchend', 126, 400);
await sleep(1500);
const parentEl = document.querySelector('.tv-under');
parentEl.dataset.mark = 'kept';
check(`before: under ${parentEl.dataset.viewId} views ${document.querySelectorAll('.tv-stack > .task-view').length}`);
await swipe(250);
await sleep(1200);
const now = document.querySelector('.tv-stack > .task-view');
check(`parent kept, not remounted: ${now === parentEl} marked ${document.querySelector('[data-mark]')?.className} now ${now?.className} ${now?.dataset.viewId}; views ${document.querySelectorAll('.tv-stack > .task-view').length}; styles left "${now?.style.visibility}${now?.querySelector('.tv-body')?.style.transform}"`);
check(`after long swipe: ${title()} back=${!!document.querySelector('.tv-back')}`);

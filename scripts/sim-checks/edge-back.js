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
const title = () => document.querySelector('.tv-sheet .tv-title, .tv-sheet textarea, .tv-sheet input')?.value ?? '?';
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
for (let i = 1; i <= 8; i++) { await sleep(16); touch('touchmove', 6 + (150 * i) / 8, 400); }
const pk = document.querySelector('.tv-peek');
check(`mid-drag: peek ${pk && getComputedStyle(pk).visibility} "${pk?.querySelector('.tv-peek-text')?.innerText}" at ${pk?.style.transform}`);
await sleep(2500);
touch('touchend', 156, 400);
await sleep(1500);
await swipe(250);
await sleep(1200);
check(`after long swipe: ${title()} back=${!!document.querySelector('.tv-back')}`);

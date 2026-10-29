// The floating menu and search: glass inside the page, dimmed and covered by
// the task view, back to normal when it closes.
const at = (sel) => {
  const r = document.querySelector(sel).getBoundingClientRect();
  const hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
  return hit.closest('.float-btn, .tv-scrim, .tv-sheet')?.className ?? hit.tagName;
};
const op = (sel) => getComputedStyle(document.querySelector(sel)).opacity;
check(`closed: menu tap hits ${at('.float-menu')}, opacity ${op('.float-menu')}`);
await sleep(2000);
document.querySelector('.task-body').click();
await sleep(2000);
check(`open: menu tap hits ${at('.float-menu')}, search ${at('.float-search')}, opacity ${op('.float-menu')}`);
await sleep(2000);
const sheet = document.querySelector('.tv-sheet');
sheet.classList.add('tv-large');
sheet.dataset.detent = 'large';
await sleep(2000);
check(`large: menu tap hits ${at('.float-menu')}`);
await sleep(1000);
document.querySelector('.tv-head .tv-close').click();
await sleep(2000);
check(`closed again: menu tap hits ${at('.float-menu')}, opacity ${op('.float-menu')}`);
document.querySelector('.float-search').click();
await sleep(2000);
check(`search: label ${document.querySelector('.float-search').getAttribute('aria-label')}`);

// A long press on a sidebar row shows its rename and delete, which replace its
// count without changing the row's height.
document.querySelector('.menu-btn').click();
await sleep(500);
const row = [...document.querySelectorAll('.sidebar .nav-card .nav-item[data-menu-id]')].find((r) => r.querySelector('.count')) ?? document.querySelector('.sidebar .nav-item[data-menu-id]');
const before = row.getBoundingClientRect().height;
const at = row.getBoundingClientRect();
const touch = new Touch({ identifier: 1, target: row, clientX: at.left + 40, clientY: at.top + 10 });
row.dispatchEvent(new TouchEvent('touchstart', { bubbles: true, touches: [touch], changedTouches: [touch] }));
await sleep(600);
row.dispatchEvent(new TouchEvent('touchend', { bubbles: true, touches: [], changedTouches: [touch] }));
const shown = [...row.querySelectorAll('.nav-actions button')].map((b) => {
  const r = b.getBoundingClientRect();
  return `${b.title} ${r.width}x${r.height}`;
});
check(`pressed ${row.classList.contains('pressed')}, actions ${getComputedStyle(row.querySelector('.nav-actions')).display}: ${shown.join(', ')}`);
check(`height ${before} -> ${row.getBoundingClientRect().height}, right edge of actions ${row.querySelector('.nav-actions').getBoundingClientRect().right} in row ${row.getBoundingClientRect().right}`);
const count = row.querySelector('.count');
check(`count ${count ? getComputedStyle(count).display : 'none on this row'}`);

// The rows' ••• and long press: the menu opens under a row with room below and
// above one near the bottom, and the row's highlight goes when the finger lifts.
document.querySelector('.menu-btn').click();
await sleep(500);
const rows = [...document.querySelectorAll('.sidebar .nav-item[data-menu-id]')];
check(`••• ${getComputedStyle(rows[0].querySelector('.nav-more')).display} ${rows[0].querySelector('.nav-more').getBoundingClientRect().width}, row height ${rows[0].getBoundingClientRect().height}`);
const press = async (row) => {
  const at = row.getBoundingClientRect();
  const touch = new Touch({ identifier: 1, target: row, clientX: at.left + 40, clientY: at.top + 10 });
  row.dispatchEvent(new TouchEvent('touchstart', { bubbles: true, touches: [touch], changedTouches: [touch] }));
  await sleep(600);
  check(`held: highlighted ${row.classList.contains('pressed')}, menu ${document.querySelector('.row-menu') ? 'open' : 'closed'}`);
  row.dispatchEvent(new TouchEvent('touchend', { bubbles: true, touches: [], changedTouches: [touch] }));
  check(`lifted: highlighted ${row.classList.contains('pressed')}, menu ${document.querySelector('.row-menu') ? 'open' : 'closed'}`);
};
const where = (row) => {
  const r = row.getBoundingClientRect(), m = document.querySelector('.row-menu').getBoundingClientRect();
  return `row ${r.top.toFixed(0)}-${r.bottom.toFixed(0)}, menu ${m.top.toFixed(0)}-${m.bottom.toFixed(0)}, right ${m.right.toFixed(0)} of row ${r.right.toFixed(0)}`;
};
await press(rows[0]);
check(`first row: ${where(rows[0])}`);
const last = rows[rows.length - 1];
document.body.dispatchEvent(new MouseEvent('click', { bubbles: true }));
await sleep(200);
last.scrollIntoView({ block: 'end' });
await sleep(200);
last.querySelector('.nav-more').click();
await sleep(200);
check(`last row: ${where(last)}`);
const card = document.querySelector('.sidebar .nav-card');
const first = card.querySelector('.nav-item').getBoundingClientRect(), box = card.getBoundingClientRect();
check(`card ${box.top.toFixed(1)}-${box.bottom.toFixed(1)}, first row top ${first.top.toFixed(1)}, last row bottom ${[...card.querySelectorAll('.nav-item')].pop().getBoundingClientRect().bottom.toFixed(1)}`);

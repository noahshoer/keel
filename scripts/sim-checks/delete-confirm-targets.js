// The delete confirmation's Cancel and Delete are full tap targets. Cancels; deletes nothing.
const size = (b) => { const r = b.getBoundingClientRect(); return `${b.textContent} ${r.width.toFixed(0)}x${r.height.toFixed(0)} at ${r.left.toFixed(0)}-${r.right.toFixed(0)}`; };
document.querySelector('.menu-btn').click();
await sleep(500);
const row = document.querySelector('.sidebar .nav-item[data-menu-id]');
row.dispatchEvent(new Event('keel-row-press'));
await sleep(200);
[...document.querySelectorAll('.row-menu .menu-item')].find((b) => b.textContent === 'Delete').click();
await sleep(300);
const m = document.querySelector('.row-menu').getBoundingClientRect();
check(`popup ${m.left.toFixed(0)}-${m.right.toFixed(0)} wide ${m.width.toFixed(0)}, ${m.top.toFixed(0)}-${m.bottom.toFixed(0)}`);
for (const b of document.querySelectorAll('.row-menu .confirm-actions .btn')) check(size(b));
await sleep(3000);
[...document.querySelectorAll('.row-menu .btn')].find((b) => b.textContent === 'Cancel').click();
await sleep(200);
check(`after Cancel: popup ${document.querySelector('.row-menu') ? 'open' : 'closed'}, row still there ${row.isConnected}`);

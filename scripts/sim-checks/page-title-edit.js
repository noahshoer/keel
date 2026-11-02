// Edit on a page turns the title into the field, in the same place at the same
// size. Leaves without saving.
document.querySelector('.menu-btn').click();
await sleep(500);
document.querySelector('.sidebar .nav-item[data-menu-id^="project:"]').click();
await sleep(900);
const box = (e) => { const r = e.getBoundingClientRect(), s = getComputedStyle(e); return `${r.left.toFixed(1)},${r.top.toFixed(1)} ${r.width.toFixed(0)}x${r.height.toFixed(1)} ${s.fontSize}/${s.fontWeight}`; };
check(`title ${box(document.querySelector('h1.view-title'))}`);
document.querySelector('.view-more').click();
await sleep(200);
[...document.querySelectorAll('.row-menu .menu-item')].find((b) => b.textContent === 'Edit').click();
await sleep(300);
const input = document.querySelector('.view-head input');
check(`field ${box(input)}, focused ${document.activeElement === input}, popup ${document.querySelector('.row-menu') ? 'open' : 'gone'}`);

// The header stays at the top; as the list scrolls the title rides up into the
// top row and shrinks, then the list slides under the header into its fade. Opens "Long list"
// (sticky-head-seed.js). Screenshots: little@9, between@12, scrolled@15.
await sleep(500);
document.querySelector('.menu-btn').click();
await sleep(1000);
[...document.querySelectorAll('.nav-item')].find((n) => n.textContent.includes('Long list')).click();
await sleep(1500);
const main = document.querySelector('.list-scroll');
const head = document.querySelector('.view-head');
const report = (at) => check(`${at}: scrollTop ${main.scrollTop}, head top ${head.getBoundingClientRect().top}, height ${head.offsetHeight}, fade ${getComputedStyle(head, '::after').transform}, title ${getComputedStyle(document.querySelector('.view-title')).scale} at ${Math.round(document.querySelector('.view-title').getBoundingClientRect().left)},${Math.round(document.querySelector('.view-title').getBoundingClientRect().top)}`);
report('rest');
main.scrollTop = 30;
await sleep(200);
report('a little');
await sleep(2800);
main.scrollTop = 150;
await sleep(200);
report('between');
await sleep(2800);
main.scrollTop = 400;
await sleep(200);
report('scrolled');
await sleep(2800);
main.scrollTop = 0;
await sleep(300);
report('back at top');
// Search slides in with the header in place, and comes back.
document.querySelector('.search-btn').click();
await sleep(1200);
check(`search: back button ${!!document.querySelector('.back-btn')}, list scrolls ${!!document.querySelector('.list-scroll')}`);
document.querySelector('.back-btn').click();
await sleep(1200);
check(`back: ${document.querySelector('.view-title').textContent}`);

// A list keeps its place through Search and back, and through the same view
// renamed; a different view opens at the top. Opens "Long list"
// (sticky-head-seed.js).
const list = () => document.querySelector('.list-scroll');
const report = (at) => check(`${at}: "${document.querySelector('.view-title').textContent}" scrollTop ${list().scrollTop}, head ${document.querySelector('.view-head').offsetHeight}`);
const open = async (text) => {
  document.querySelector('.menu-btn').click();
  await sleep(800);
  [...document.querySelectorAll('.nav-item')].find((n) => n.textContent.includes(text)).click();
  await sleep(1200);
};
await open('Long list');
list().scrollTop = 400;
await sleep(500);
report('scrolled');
document.querySelector('.search-btn').click();
await sleep(1200);
check(`in search: scrollTop ${list().scrollTop}`);
document.querySelector('.back-btn').click();
await sleep(1200);
report('back from search');
await open('Today');
report('a different view');
await open('Long list');
report('back to it from the drawer');

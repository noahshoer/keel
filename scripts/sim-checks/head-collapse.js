// The collapsing header's edge cases: long names beside the icons, the date
// line at rest, Search, and switching views while scrolled.
// Screenshots: today@6, long@10, word@14, search@18, switched@23.
const open = async (text) => {
  document.querySelector('.menu-btn').click();
  await sleep(800);
  [...document.querySelectorAll('.nav-item')].find((n) => n.textContent.includes(text)).click();
  await sleep(1200);
};
const content = () => document.querySelector('.content');
const report = (at) => {
  const t = document.querySelector('.view-title').getBoundingClientRect();
  const icon = (document.querySelector('.view-toggle') ?? document.querySelector('.search-btn')).getBoundingClientRect();
  const l = document.querySelector('.list-scroll');
  check(`${at}: "${document.querySelector('.view-title').textContent}" right ${Math.round(t.right)}, icons at ${Math.round(icon.left)}, clear ${t.right <= icon.left}, scrollTop ${l.scrollTop}, collapse ${document.querySelector('.view-head').style.getPropertyValue('--head-collapse')}, head ${document.querySelector('.view-head').offsetHeight}`);
};
await sleep(1000);
report('today at rest');
await sleep(2000);
await open('Home improvement projects for');
document.querySelector('.view-head').style.setProperty('--title-move', '1');
await sleep(300);
report('long name, collapsed');
await sleep(1700);
document.querySelector('.view-head').style.removeProperty('--title-move');
await open('Supercali');
document.querySelector('.view-head').style.setProperty('--title-move', '1');
await sleep(300);
report('one word, collapsed');
await sleep(1500);
document.querySelector('.search-btn').click();
await sleep(1500);
const h = document.querySelector('.view-head').getBoundingClientRect();
const sub = document.querySelector('.list-body').firstElementChild?.getBoundingClientRect();
check(`search: head ${Math.round(h.height)} tall, first item at ${sub && Math.round(sub.top)}, title row ${getComputedStyle(content()).getPropertyValue('--title-row')}`);
await sleep(1500);
document.querySelector('.back-btn').click();
await sleep(1200);
await open('Long list');
document.querySelector('.list-scroll').scrollTop = 400;
await sleep(300);
report('scrolled');
await open('Today');
report('switched to Today');
await open('Inbox');
report('then Inbox');

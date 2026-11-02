// The sidebar's add form has the shape of the cards under it: same edges and
// corners, a card of one row. The search field shares those edges.
const edges = (sel) => {
  const e = document.querySelector(sel);
  const b = e.getBoundingClientRect();
  return `${sel} left ${b.left.toFixed(1)} right ${b.right.toFixed(1)} height ${b.height.toFixed(1)} radius ${getComputedStyle(e).borderRadius}`;
};
document.querySelector('.menu-btn').click();
await sleep(600);
[...document.querySelectorAll('.section-head button')][0].click();
await sleep(1200);
check(edges('.sidebar-search'));
check(edges('.project-form input'));
check(edges('.nav-card'));
check(edges('.section-head'));

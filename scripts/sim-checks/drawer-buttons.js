// The drawer's theme and settings buttons slide in and out with the drawer
// (slowed to 4 s so screenshots catch it mid-slide).
const sb = document.querySelector('.sidebar');
sb.style.transitionDuration = '4s';
sb.style.transitionTimingFunction = 'linear';
const where = () => {
  const dx = new DOMMatrix(getComputedStyle(sb).transform).m41;
  const t = document.querySelector('.brand .theme-toggle').getBoundingClientRect();
  return `drawer shift ${Math.round(dx)}, theme button x ${Math.round(t.x)}`;
};
await sleep(1000);
document.querySelector('.menu-btn').click();
check('opening');
for (let i = 0; i < 4; i++) { await sleep(1000); check(where()); }
await sleep(1000);
document.querySelector('.drawer-scrim').click();
check('closing');
for (let i = 0; i < 4; i++) { await sleep(1000); check(where()); }

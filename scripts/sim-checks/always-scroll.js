// A short list still scrolls: `.main` overflows by a pixel on every view, and
// a real drag (sent from outside, within the 12 s wait) pulls it past its top.
const main = document.querySelector('.main');
const report = (name) => check(`${name}: overflow ${main.scrollHeight - main.clientHeight}px`);
await sleep(500);
report(document.querySelector('.view-title').textContent);
let least = 0;
main.addEventListener('scroll', () => { least = Math.min(least, main.scrollTop); });
await sleep(12000);
check(`drag: scrollTop reached ${least}, rests at ${main.scrollTop}`);
for (const name of ['Inbox', 'Upcoming', 'Groceries']) {
  document.querySelector('.menu-btn').click();
  await sleep(1000);
  [...document.querySelectorAll('.nav-item')].find((n) => n.textContent.includes(name)).click();
  await sleep(1500);
  report(name);
}

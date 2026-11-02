// The date card's calendar: opens it, then reports what's at the top every 2 s
// for a minute, so swipes from outside (the simulator's touch input) show up.
const byText = (sel, t) => [...document.querySelectorAll(sel)].find((e) => e.textContent.includes(t));
document.querySelector('.add-task-row.view-add').click();
await sleep(800);
document.querySelector('button[title="Date"]').click();
await sleep(500);
byText('.menu-item', 'Pick a date').click();
await sleep(800);
const report = () => {
  const s = document.querySelector('.cal-scroll');
  const title = document.querySelector('.cal-title');
  const head = document.querySelector('.sched-head').getBoundingClientRect();
  const size = getComputedStyle(title).fontSize;
  check(`${title.textContent} top=${Math.round(s.scrollTop)} head=${Math.round(head.height)} title=${size} weeks=${document.querySelectorAll('.cal-week').length} shown=${document.querySelectorAll('.cal-day:not(.other)').length}`);
};
for (let i = 0; i < 30; i += 1) {
  report();
  await sleep(2000);
}

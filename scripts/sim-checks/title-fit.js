// Opens each long-named project (title-fit-seed.js) through the drawer and
// reports how its title fits: one line, beneath the icons, cut between words.
// Screenshots: today@6, then name@10 and every 4 s.
const names = ['Groceries', 'Home improvement projects', 'Apartment renovation and furniture shopping', 'Interdepartmental reorganization', 'Home improvement projects for the spring and summer, plus everything we put off last year', 'Supercalifragilisticexpialidocious'];
await sleep(3000);
const report = (name) => {
  const t = document.querySelector('.view-title');
  const css = getComputedStyle(t);
  const lines = Math.round(t.clientHeight / parseFloat(css.lineHeight));
  const cut = t.textContent.endsWith('…') ? t.textContent.slice(0, -1) : t.textContent;
  const atWord = cut === name || name[cut.length] === ' ';
  const icon = document.querySelector('.search-btn').getBoundingClientRect();
  check(`"${t.textContent}" ${css.fontSize} ${lines} line(s), width ${t.clientWidth}, overflowing ${t.scrollWidth > t.clientWidth}, at a word ${atWord}, below the icons ${t.getBoundingClientRect().top >= icon.bottom}, aria ${t.getAttribute('aria-label') !== null}, page overflow ${document.documentElement.scrollWidth > innerWidth}`);
};
report(document.querySelector('.view-title').textContent);
for (const name of names) {
  document.querySelector('.menu-btn').click();
  await sleep(1000);
  const item = [...document.querySelectorAll('.nav-item')].find((n) => n.querySelector('.nav-name')?.textContent === name);
  if (!item) { check(`MISSING ${name}`); await sleep(3000); continue; }
  item.click();
  await sleep(2000);
  report(name);
  await sleep(1000);
}

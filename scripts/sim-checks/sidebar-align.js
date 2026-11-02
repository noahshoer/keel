// The sidebar's headings start at the logo's left edge, and their plus icons
// line up under the settings icon.
await sleep(500);
document.querySelector('.menu-btn')?.click();
await sleep(1000);
const box = (e) => e.getBoundingClientRect();
const logo = box(document.querySelector('.brand-mark'));
const gear = box(document.querySelector('.settings-btn svg'));
for (const head of document.querySelectorAll('.section-head')) {
  const range = document.createRange();
  range.selectNodeContents(head.firstChild);
  const plus = box(head.querySelector('svg'));
  check(`${head.textContent}: text left ${(box(range).left - logo.left).toFixed(1)} from the logo, plus center ${((plus.left + plus.right) / 2 - (gear.left + gear.right) / 2).toFixed(1)} from the gear's`);
}

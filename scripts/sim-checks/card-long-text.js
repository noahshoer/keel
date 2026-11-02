// The task card with long text: it stops at the top of what's visible, the text
// scrolls and the pills stay, and an open picker gets its room back. A usual card
// (title, chips, one line of description) doesn't change when a picker opens.
const set = (el, value) => {
  Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value").set.call(el, value);
  el.dispatchEvent(new Event("input", { bubbles: true }));
};
const text = () => document.querySelector(".editor-text");
const card = () => document.querySelector("form.editor");
const title = () => document.querySelector(".title-input");
const desc = () => document.querySelector("textarea.desc");
const priority = () => document.querySelector('.chip-btn[title="Priority"]');

document.querySelector(".add-task-row.view-add")?.click();
await sleep(1500);
set(title(), "Call mom tomorrow p1");
await sleep(300);
const usual = text().clientHeight;
priority().click();
await sleep(500);
check(`usual card: ${usual}pt, with a picker ${text().clientHeight}pt, unchanged ${usual === text().clientHeight}`);
priority().click();
await sleep(300);

set(title(), "A long title that wraps over a good many lines of the card. ".repeat(6));
set(desc(), Array.from({ length: 30 }, (_, i) => `Line ${i + 1}`).join("\n"));
await sleep(500);
const safeTop = Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--safe-top")) || 0;
const box = card().getBoundingClientRect();
const pills = document.querySelector(".editor-actions").getBoundingClientRect();
check(`long card: top ${Math.round(box.top)}pt, below the status bar ${box.top >= safeTop}`);
check(`pills inside the card ${pills.bottom <= box.bottom + 0.5}, above the keyboard ${box.bottom <= visualViewport.height + visualViewport.offsetTop}`);
check(`text scrolls ${text().scrollHeight > text().clientHeight} (${text().scrollHeight} in ${text().clientHeight})`);
text().scrollTop = 1e6;
await sleep(100);
check(`reaches the end ${Math.abs(text().scrollHeight - text().clientHeight - text().scrollTop) < 1}`);
priority().click();
await sleep(500);
const pop = document.querySelector(".popover").getBoundingClientRect();
const under = card().getBoundingClientRect();
check(`with a picker: text ${text().clientHeight}pt, picker clear of the card ${pop.bottom <= under.top}, on screen ${pop.top >= safeTop}`);
priority().click();
await sleep(300);
check(`picker closed: text back to ${text().clientHeight}pt`);

// Every picker, above a full card with the keyboard up: does it fit without scrolling?
for (const name of ["Date", "Deadline: the date it must be done by", "Priority", "Project / section", "Labels", "Reminders", "Repeat"]) {
  const chip = document.querySelector(`.chip-btn[title="${name}"]`);
  if (!chip) { check(`${name}: no pill`); continue; }
  chip.scrollIntoView({ inline: "center", block: "nearest" });
  chip.click();
  await sleep(500);
  const p = document.querySelector(".popover");
  const clip = p ? p.scrollHeight - p.clientHeight : -1;
  const kb = visualViewport.height < document.documentElement.clientHeight - 0.5;
  check(`${name}: picker ${p ? Math.round(p.getBoundingClientRect().height) : "none"}pt, cut off by ${clip}pt, text ${text().clientHeight}pt, keyboard ${kb ? "up" : "down"}`);
  chip.click();
  await sleep(400);
}

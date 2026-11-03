// Adding a task with a streak, against the real database: typed ("every day
// streak 30"), from the pill, and "streak" with no repeat, which stays a word.
const stamp = String(Date.now()).slice(-5);
const title = () => document.querySelector('.editor .title-input');
const pill = () => document.querySelector('.editor button[title="Streak"]');
const typeIt = (text) => {
  const el = title();
  Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value').set.call(el, text);
  el.dispatchEvent(new Event('input', { bubbles: true }));
};
const tasks = () => window.__TAURI__.core.invoke('all_tasks', { args: {} });
const saved = async (name) => {
  const t = (await tasks()).find((x) => x.content === name);
  return t ? `${name}: tracked ${t.streak_enabled}, goal ${t.streak_goal}, repeats ${!!t.due?.is_recurring}` : `${name}: NOT FOUND`;
};
const submit = () => document.querySelector('.editor .btn-save').click();
document.querySelector('.add-task-row').click();
await sleep(800);

typeIt(`Typed ${stamp} streak`);
await sleep(300);
check(`no repeat: pill ${pill() === null ? 'absent' : 'SHOWN'}`);
typeIt(`Typed ${stamp} every day streak 30`);
await sleep(400);
check(`typed: pill "${pill()?.textContent}", marks ${[...document.querySelectorAll('.title-mirror mark')].map((m) => m.textContent).join(' | ')}`);
await sleep(1500);
submit();
await sleep(1000);
check(await saved(`Typed ${stamp}`));

typeIt(`Pill ${stamp} every day`);
await sleep(400);
check(`offered: pill "${pill()?.textContent}"`);
pill().click();
await sleep(300);
check(`tapped: pill "${pill()?.textContent}"`);
submit();
await sleep(1000);
check(await saved(`Pill ${stamp}`));

typeIt(`Word ${stamp} streak 30`);
await sleep(300);
submit();
await sleep(1000);
check(await saved(`Word ${stamp} streak 30`));

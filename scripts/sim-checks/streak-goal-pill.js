// The Streak pill's popover while adding a task, against the real database: a
// tap turns it on and focuses the goal, typing shows on the pill, and the goal saves.
const stamp = String(Date.now()).slice(-5);
const title = () => document.querySelector('.editor .title-input');
const pill = () => document.querySelector('.editor button[title="Streak"]');
const goal = () => document.querySelector('.streak-goal-pop input');
const set = (el, proto, text) => {
  Object.getOwnPropertyDescriptor(proto, 'value').set.call(el, text);
  el.dispatchEvent(new Event('input', { bubbles: true }));
};
const tasks = () => window.__TAURI__.core.invoke('all_tasks', { args: {} });
document.querySelector('.add-task-row').click();
await sleep(800);
set(title(), HTMLTextAreaElement.prototype, `Goal ${stamp} every day`);
await sleep(600);
check(`offered: "${pill()?.textContent}", popover ${goal() ? 'OPEN' : 'closed'}`);
pill().click();
await sleep(600);
const pop = document.querySelector('.streak-goal-pop').getBoundingClientRect();
const card = document.querySelector('.editor').getBoundingClientRect();
check(`tapped: "${pill().textContent}", goal focused ${document.activeElement === goal()}`);
check(`popover ${Math.round(pop.left)}..${Math.round(pop.right)} of ${innerWidth}, bottom ${Math.round(pop.bottom)}, card top ${Math.round(card.top)}`);
set(goal(), HTMLInputElement.prototype, '30');
await sleep(2000);
check(`typed: "${pill().textContent}", title "${title().value}"`);
await sleep(3000);
document.querySelector('.editor .btn-save').click();
await sleep(1000);
const t = (await tasks()).find((x) => x.content === `Goal ${stamp}`);
check(t ? `saved: tracked ${t.streak_enabled}, goal ${t.streak_goal}, repeats ${!!t.due?.is_recurring}` : 'NOT FOUND');

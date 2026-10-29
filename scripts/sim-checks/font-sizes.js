// Text sizes: list rows vs the task view's sub-task rows and title.
const f = (sel) => { const e = document.querySelector(sel); if (!e) return '-'; const s = getComputedStyle(e); return `${s.fontSize}/${s.fontWeight}`; };
check(`list row name ${f('.task-row .task-content')}, list count ${f('.task-row .task-count')}`);
document.querySelector('.task-body').click();
await sleep(2500);
check(`view title ${f('.tv-title textarea')}, field row ${f('.tv-field-btn')}, sub-task name ${f('.tv-sub-body .task-content')}, sub-task count ${f('.tv-sub-body .task-count')}, add sub-task ${f('.tv-add-sub')}`);

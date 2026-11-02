// Adds a project with enough tasks to scroll, for sticky-head.js. Run once.
const send = (action, temp) => window.__TAURI__.core.invoke('dispatch', { args: { cmd: { uuid: crypto.randomUUID(), temp_id: temp ?? null, action } } });
const temp = `tmp-${crypto.randomUUID()}`;
const added = await send({ type: 'project_add', name: 'Long list' }, temp);
const project_id = added.id_mapping[temp];
for (let i = 1; i <= 30; i++) await send({ type: 'task_add', content: `Task number ${i}`, project_id });
check(`added 30 tasks to ${project_id}`);

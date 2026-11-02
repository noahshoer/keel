// Adds long-named projects for title-fit.js. Run once on a test simulator.
const names = [
  'Groceries',
  'Home improvement projects',
  'Apartment renovation and furniture shopping',
  'Interdepartmental reorganization',
  'Home improvement projects for the spring and summer, plus everything we put off last year',
  'Supercalifragilisticexpialidocious',
];
for (const name of names) {
  const cmd = { uuid: crypto.randomUUID(), temp_id: `tmp-${crypto.randomUUID()}`, action: { type: 'project_add', name } };
  await window.__TAURI__.core.invoke('dispatch', { args: { cmd } });
  check(`added ${name}`);
}

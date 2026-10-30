// In-memory stand-in for the Tauri backend, for `pnpm dev` in a plain
// browser. Loaded only when `import.meta.env.DEV` (see invoke.ts). It does
// not port domain logic: dispatch sets fields directly, and views return
// fixture data (recurrence, filters and grouping stay in keel-core).
import type { Task } from "../bindings";
import type { Handlers } from "./commands";
import * as fx from "./fixtures";

const state = {
  tasks: structuredClone(fx.tasks),
  reminderSettings: structuredClone(fx.reminderSettings),
  diag: false,
  nextId: 1,
};

const open = () => state.tasks.filter((t) => !t.checked && !t.is_deleted);

function newTask(id: string, content: string, projectId: string): Task {
  return fx.makeTask({ id, content, project_id: projectId, added_at: new Date().toISOString() });
}

export const mockHandlers: Handlers = {
  list_projects: async () => fx.projects,
  list_tasks: async ({ project_id }) => open().filter((t) => t.project_id === project_id),
  today_view: async () => open().filter((t) => t.due?.date.startsWith(fx.today) === true),
  all_tasks: async () => open(),
  upcoming_view: async () => open().filter((t) => t.due !== null),
  project_view: async ({ project_id }) => open().filter((t) => t.project_id === project_id),
  list_labels: async () => fx.labels,
  label_view: async ({ name }) => open().filter((t) => t.labels.includes(name)),
  completed_today: async () => state.tasks.filter((t) => t.checked && !t.is_deleted),
  completed_in_project: async ({ project_id }) =>
    state.tasks.filter((t) => t.checked && !t.is_deleted && t.project_id === project_id),
  completed_with_label: async ({ name }) =>
    state.tasks.filter((t) => t.checked && !t.is_deleted && t.labels.includes(name)),
  list_sections: async ({ project_id }) => fx.sections.filter((s) => s.project_id === project_id),
  list_filters: async () => fx.filters,
  check_filter: async () => null,
  filter_view: async ({ query }) => [{ query, tasks: open() }],
  dispatch: async ({ cmd }) => {
    const idMapping: Record<string, string> = {};
    const a = cmd.action;
    if (a.type === "task_add") {
      const id = `mock-${state.nextId++}`;
      const t = newTask(id, a.content, a.project_id);
      if (a.priority != null) t.priority = a.priority;
      if (a.description != null) t.description = a.description;
      if (a.labels != null) t.labels = a.labels;
      if (a.due != null) t.due = a.due;
      state.tasks.push(t);
      if (cmd.temp_id != null) idMapping[cmd.temp_id] = id;
    } else if (a.type === "task_complete") {
      const t = state.tasks.find((x) => x.id === a.id);
      if (t) {
        t.checked = true;
        t.completed_at = new Date().toISOString();
      }
    } else if (a.type === "task_update") {
      const t = state.tasks.find((x) => x.id === a.id);
      if (t) {
        if (a.content != null) t.content = a.content;
        if (a.description != null) t.description = a.description;
        if (a.priority != null) t.priority = a.priority;
        if (a.labels != null) t.labels = a.labels;
        if (a.is_collapsed != null) t.is_collapsed = a.is_collapsed;
      }
    }
    return { id_mapping: idMapping };
  },
  search: async ({ query }) => {
    const q = query.toLowerCase();
    const match = (t: Task) => t.content.toLowerCase().includes(q);
    return {
      open: open().filter(match),
      completed: state.tasks.filter((t) => t.checked && !t.is_deleted && match(t)),
    };
  },
  list_reminders: async () => fx.reminders,
  import_todoist_csv: async ({ project_name }) => ({
    project_id: "p-import",
    project_name,
    tasks: 0,
    sections: 0,
    labels_created: 0,
    comments: 0,
    kept_as_text: 0,
  }),
  get_reminder_settings: async () => state.reminderSettings,
  set_reminder_settings: async ({ settings }) => {
    state.reminderSettings = settings;
    return null;
  },
  upcoming_reminders: async () => [],
  notification_permission: async () => "granted",
  request_notification_permission: async () => "granted",
  reminders_when_closed: async () => false,
  test_notification: async () => null,
  diag_enabled: async () => state.diag,
  set_diag_enabled: async ({ on }) => {
    state.diag = on;
    return null;
  },
  diag_log: async () => null,
};

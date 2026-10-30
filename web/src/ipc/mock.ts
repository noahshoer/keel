// In-memory stand-in for the Tauri backend, for `pnpm dev` in a plain
// browser and for tests. Loaded only when `import.meta.env.DEV` (see
// invoke.ts). It does not port domain logic: dispatch sets fields directly,
// and the dated views return fixture ids (recurrence, filters and grouping
// stay in keel-core). Every handler returns a copy, and `resetMock()` (run
// before each test, src/test/setup.ts) starts over from the fixtures.
import type { Action, Due, DueEdit, Filter, Label, Project, Section, Task } from "../bindings";
import type { CommandName, Handlers } from "./commands";
import * as fx from "./fixtures";

function fresh() {
  return {
    tasks: structuredClone(fx.tasks),
    projects: structuredClone(fx.projects),
    sections: structuredClone(fx.sections),
    labels: structuredClone(fx.labels),
    filters: structuredClone(fx.filters),
    reminders: structuredClone(fx.reminders),
    reminderSettings: structuredClone(fx.reminderSettings),
    diag: false,
    nextId: 1,
  };
}

let state = fresh();

/** Back to the fixtures (tests call this before each case). */
export function resetMock(): void {
  state = fresh();
}

const live = (t: Task) => !t.is_deleted;
const open = (t: Task) => !t.checked && !t.is_deleted;
const byId = (ids: readonly string[]): Task[] => ids.flatMap((id) => state.tasks.filter((t) => t.id === id));
const openById = (ids: readonly string[]): Task[] => byId(ids).filter(open);
const done = (): Task[] => state.tasks.filter((t) => t.checked && live(t));

function find<T extends { id: string; is_deleted: boolean }>(rows: T[], id: string): T {
  const row = rows.find((r) => r.id === id && !r.is_deleted);
  if (!row) throw `not found: ${id}`;
  return row;
}

function dueFrom(edit: DueEdit): Due | null {
  if (edit.date == null) return null;
  return {
    date: edit.date,
    timezone: null,
    string: edit.date,
    lang: "en",
    is_recurring: edit.recurrence != null,
    recurrence: edit.recurrence ?? null,
  };
}

/** The task and everything under it. */
function subtree(id: string): Task[] {
  const out = state.tasks.filter((t) => t.id === id);
  for (let i = 0; i < out.length; i++) {
    const parent = out[i]?.id;
    out.push(...state.tasks.filter((t) => t.parent_id === parent));
  }
  return out;
}

function newId(prefix: string): string {
  return `mock-${prefix}-${state.nextId++}`;
}

/** Applies one action; returns the new row's id for creates. */
function apply(a: Action): string | null {
  const now = new Date().toISOString();
  switch (a.type) {
    case "task_add": {
      const id = newId("t");
      state.tasks.push(
        fx.makeTask({
          id,
          content: a.content,
          project_id: a.project_id,
          description: a.description ?? "",
          priority: a.priority ?? "None",
          due: a.due ?? null,
          labels: a.labels ?? [],
          parent_id: a.parent_id ?? null,
          section_id: a.section_id ?? null,
          deadline: a.deadline ?? null,
          added_at: now,
        }),
      );
      return id;
    }
    case "task_update": {
      const t = find(state.tasks, a.id);
      if (a.content != null) t.content = a.content;
      if (a.description != null) t.description = a.description;
      if (a.priority != null) t.priority = a.priority;
      if (a.project_id != null) t.project_id = a.project_id;
      if (a.labels != null) t.labels = a.labels;
      if (a.due != null) t.due = dueFrom(a.due);
      if (a.is_collapsed != null) t.is_collapsed = a.is_collapsed;
      if (a.deadline != null) t.deadline = a.deadline === "" ? null : a.deadline;
      return null;
    }
    case "task_move": {
      const t = find(state.tasks, a.id);
      t.project_id = a.project_id;
      t.section_id = a.section_id ?? null;
      return null;
    }
    case "task_complete": {
      for (const t of subtree(find(state.tasks, a.id).id)) {
        t.checked = true;
        t.completed_at = now;
      }
      return null;
    }
    case "task_uncomplete": {
      const t = find(state.tasks, a.id);
      t.checked = false;
      t.completed_at = null;
      return null;
    }
    case "task_delete": {
      for (const t of subtree(find(state.tasks, a.id).id)) t.is_deleted = true;
      return null;
    }
    case "task_set_streak": {
      const t = find(state.tasks, a.id);
      t.streak_enabled = a.enabled;
      t.streak_goal = a.goal ?? null;
      return null;
    }
    case "project_add": {
      const id = newId("p");
      const base = state.projects[0];
      if (!base) throw "no projects";
      state.projects.push({ ...structuredClone(base), id, name: a.name, is_inbox: false });
      return id;
    }
    case "project_update":
      find(state.projects, a.id).name = a.name;
      return null;
    case "project_delete":
      find(state.projects, a.id).is_deleted = true;
      for (const t of state.tasks) if (t.project_id === a.id) t.is_deleted = true;
      return null;
    case "section_add": {
      const id = newId("s");
      const base = fx.sections[0];
      if (!base) throw "no sections";
      state.sections.push({ ...structuredClone(base), id, project_id: a.project_id, name: a.name });
      return id;
    }
    case "section_update":
      find(state.sections, a.id).name = a.name;
      return null;
    case "section_delete":
      find(state.sections, a.id).is_deleted = true;
      for (const t of state.tasks) if (t.section_id === a.id) t.is_deleted = true;
      return null;
    case "label_add": {
      const id = newId("l");
      state.labels.push({
        id,
        name: a.name,
        color: "grey",
        item_order: 0,
        is_favorite: false,
        is_deleted: false,
      });
      return id;
    }
    case "label_update": {
      const label = find(state.labels, a.id);
      for (const t of state.tasks) t.labels = t.labels.map((n) => (n === label.name ? a.name : n));
      label.name = a.name;
      return null;
    }
    case "label_delete": {
      const label = find(state.labels, a.id);
      label.is_deleted = true;
      for (const t of state.tasks) t.labels = t.labels.filter((n) => n !== label.name);
      return null;
    }
    case "filter_add": {
      const id = newId("f");
      state.filters.push({
        id,
        name: a.name,
        query: a.query,
        color: "grey",
        item_order: 0,
        is_favorite: false,
        is_deleted: false,
      });
      return id;
    }
    case "filter_update": {
      const f = find(state.filters, a.id);
      if (a.name != null) f.name = a.name;
      if (a.query != null) f.query = a.query;
      return null;
    }
    case "filter_delete":
      find(state.filters, a.id).is_deleted = true;
      return null;
    case "reminder_add": {
      const id = newId("r");
      state.reminders.push({
        id,
        task_id: a.task_id,
        kind: a.minute_offset != null ? "Relative" : "Absolute",
        due: a.at != null ? dueFrom({ date: a.at }) : null,
        minute_offset: a.minute_offset ?? null,
        is_deleted: false,
      });
      return id;
    }
    case "reminder_delete":
      find(state.reminders, a.id).is_deleted = true;
      return null;
  }
}

const alive = <T extends { is_deleted: boolean }>(rows: T[]): T[] => rows.filter((r) => !r.is_deleted);

function checkQuery(query: string): void {
  const problem = fx.badQueries[query];
  if (problem !== undefined) throw problem;
}

const handlers: Handlers = {
  list_projects: async (): Promise<Project[]> => alive(state.projects),
  // Plain id lookup: every open task in the project.
  list_tasks: async ({ project_id }) => state.tasks.filter((t) => t.project_id === project_id && open(t)),
  today_view: async () => openById(fx.viewIds.today),
  all_tasks: async () => state.tasks.filter(live),
  upcoming_view: async () => openById(fx.viewIds.upcoming),
  project_view: async ({ project_id }) => state.tasks.filter((t) => t.project_id === project_id && open(t)),
  list_labels: async (): Promise<Label[]> => alive(state.labels),
  label_view: async ({ name }) => openById(fx.viewIds.label[name] ?? []),
  completed_today: async () => done(),
  completed_in_project: async ({ project_id }) => done().filter((t) => t.project_id === project_id),
  completed_with_label: async ({ name }) => done().filter((t) => t.labels.includes(name)),
  list_sections: async ({ project_id }): Promise<Section[]> =>
    alive(state.sections).filter((s) => s.project_id === project_id),
  list_filters: async (): Promise<Filter[]> => alive(state.filters),
  check_filter: async ({ query }) => {
    checkQuery(query);
    return null;
  },
  filter_view: async ({ query }) => {
    checkQuery(query);
    return query.split(",").map((q) => ({ query: q.trim(), tasks: openById(fx.viewIds.filter) }));
  },
  dispatch: async ({ cmd }) => {
    const id = apply(cmd.action);
    const id_mapping: Record<string, string> = {};
    if (id !== null && cmd.temp_id != null) id_mapping[cmd.temp_id] = id;
    return { id_mapping };
  },
  search: async ({ query }) => {
    const ids = fx.searchResults[query.trim()] ?? {
      open: fx.viewIds.searchOpen,
      completed: fx.viewIds.searchCompleted,
    };
    return { open: openById(ids.open), completed: byId(ids.completed).filter(live) };
  },
  list_reminders: async () => alive(state.reminders),
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

/** Each handler, returning a copy so callers can't change the mock's state. */
function copying(h: Handlers): Handlers {
  const out: Partial<Record<CommandName, unknown>> = {};
  for (const name of Object.keys(h) as CommandName[]) {
    const run = h[name] as (args: unknown) => Promise<unknown>;
    out[name] = async (args: unknown) => structuredClone(await run(args));
  }
  return out as Handlers;
}

export const mockHandlers: Handlers = copying(handlers);

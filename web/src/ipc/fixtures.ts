// Sample data for the browser mock (dev only). Typed against the bindings so
// a changed Rust type fails the build here.
import type { Filter, Label, Project, Reminder, ReminderSettings, Section, Task } from "../bindings";

function localDate(offsetDays: number): string {
  const d = new Date();
  d.setDate(d.getDate() + offsetDays);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

export const today = localDate(0);
const NOW = `${today}T08:00:00Z`;

const project = (p: Partial<Project> & Pick<Project, "id" | "name" | "color">): Project => ({
  icon: null,
  parent_id: null,
  child_order: 0,
  order_key: null,
  view_style: "List",
  is_favorite: false,
  is_collapsed: false,
  is_archived: false,
  is_inbox: false,
  is_deleted: false,
  added_at: NOW,
  updated_at: NOW,
  ...p,
});

export const makeTask = (t: Partial<Task> & Pick<Task, "id" | "project_id" | "content">): Task => ({
  section_id: null,
  parent_id: null,
  description: "",
  priority: "None",
  due: null,
  deadline: null,
  duration: null,
  labels: [],
  child_order: 0,
  order_key: null,
  day_order: 0,
  is_collapsed: false,
  checked: false,
  is_deleted: false,
  added_at: NOW,
  completed_at: null,
  updated_at: NOW,
  streak_enabled: false,
  streak_current: 0,
  streak_best: 0,
  streak_goal: null,
  streak_last_period: null,
  ...t,
});

const dueOn = (date: string, text: string): Task["due"] => ({
  date,
  timezone: null,
  string: text,
  lang: "en",
  is_recurring: false,
});

export const projects: Project[] = [
  project({ id: "p-inbox", name: "Inbox", color: "grey", is_inbox: true }),
  project({ id: "p-home", name: "Home", color: "green", child_order: 1, is_favorite: true }),
];

export const sections: Section[] = [
  {
    id: "s-chores",
    project_id: "p-home",
    name: "Chores",
    description: null,
    section_order: 0,
    order_key: null,
    is_collapsed: false,
    is_archived: false,
    is_deleted: false,
    added_at: NOW,
    updated_at: NOW,
  },
];

export const tasks: Task[] = [
  makeTask({ id: "t-inbox", project_id: "p-inbox", content: "Call the dentist", priority: "P2" }),
  makeTask({
    id: "t-taxes",
    project_id: "p-inbox",
    content: "File the taxes",
    due: dueOn(localDate(-2), "2 days ago"),
    child_order: 1,
  }),
  makeTask({
    id: "t-laundry",
    project_id: "p-home",
    section_id: "s-chores",
    content: "Do laundry",
    due: dueOn(today, "today"),
    labels: ["errand"],
    priority: "P1",
  }),
  makeTask({
    id: "t-sheets",
    project_id: "p-home",
    section_id: "s-chores",
    parent_id: "t-laundry",
    content: "Wash the sheets",
    due: dueOn(today, "today"),
  }),
  makeTask({
    id: "t-groceries",
    project_id: "p-home",
    content: "Buy groceries",
    due: dueOn(localDate(1), "tomorrow"),
    child_order: 1,
  }),
  makeTask({
    id: "t-done",
    project_id: "p-home",
    content: "Take out the bins",
    checked: true,
    completed_at: NOW,
  }),
];

// Hand-written view results: "what keel-core would return for this data".
// The mock never computes a view; it looks these ids up.
export const viewIds = {
  /** Today: the overdue task, then the two due today. */
  today: ["t-taxes", "t-laundry", "t-sheets"],
  /** Upcoming's window (from today): today's tasks, then tomorrow's. */
  upcoming: ["t-laundry", "t-sheets", "t-groceries"],
  label: { errand: ["t-laundry"] } as Record<string, string[]>,
  /** The "Important" filter (p1). */
  filter: ["t-laundry"],
  /** Search results, whatever the query. */
  searchOpen: ["t-laundry", "t-sheets"],
  searchCompleted: ["t-done"],
};

/** Search queries with their own results (anything else gets searchOpen / searchCompleted). */
export const searchResults: Record<string, { open: string[]; completed: string[] }> = {
  bins: { open: [], completed: ["t-done"] },
  nothing: { open: [], completed: [] },
};

/** Queries the mock's filter check rejects, with the backend's message. */
export const badQueries: Record<string, string> = { "p1 &": "expected a term after '&'" };

export const labels: Label[] = [
  { id: "l-errand", name: "errand", color: "orange", item_order: 0, is_favorite: false, is_deleted: false },
];

export const filters: Filter[] = [
  {
    id: "f-p1",
    name: "Important",
    query: "p1",
    color: "red",
    item_order: 0,
    is_favorite: false,
    is_deleted: false,
  },
  {
    id: "f-bad",
    name: "Broken",
    query: "p1 &",
    color: "grey",
    item_order: 1,
    is_favorite: false,
    is_deleted: false,
  },
];

export const reminders: Reminder[] = [
  {
    id: "r-laundry",
    task_id: "t-laundry",
    kind: "Relative",
    due: null,
    minute_offset: 30,
    is_deleted: false,
  },
];

export const reminderSettings: ReminderSettings = {
  enabled: true,
  offset_minutes: 0,
  all_day_time: "09:00",
  daily_summary: null,
};

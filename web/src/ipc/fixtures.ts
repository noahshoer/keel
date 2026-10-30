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
];

/** Ids the mock's Today view returns (open tasks due today). */
export const todayTaskIds: string[] = ["t-laundry", "t-sheets"];

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

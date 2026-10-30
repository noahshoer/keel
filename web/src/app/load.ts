// What the main pane and sidebar show, loaded in one place for the current
// view: the same calls and branches as the Dioxus App's loader
// (crates/ui/src/main.rs `Loaded`).
import { useEffect, useRef, useState } from "react";
import type { Filter, FilterGroup, Label, Project, Reminder, Section, Task } from "../bindings";
import { localToday } from "../clock";
import { upcomingTasks } from "../core";
import { invoke } from "../ipc/invoke";
import { UPCOMING_DAYS, type View } from "./view";

export type Loaded = {
  /** The view this data is for (the pane shows this, not the view just picked). */
  view: View;
  projects: Project[];
  /** Sections of every project. */
  sections: Section[];
  labels: Label[];
  filters: Filter[];
  inbox: Project;
  todayCount: number;
  /** Every task reminder (for the rows' bell chips). */
  reminders: Reminder[];
  /** The view's tasks. */
  tasks: Task[];
  /** Every non-deleted task, open and checked (card heads and counts). */
  all: Task[];
  /** Empty unless "Show completed" is on for the view. */
  completed: Task[];
  /** A filter view's lists (one per comma group). */
  filterGroups: FilterGroup[];
  /** Why a saved filter couldn't run. */
  filterError: string | null;
};

async function viewTasks(
  view: View,
  inbox: Project,
  todayTasks: Task[],
  today: string,
): Promise<{ tasks: Task[]; filterGroups: FilterGroup[]; filterError: string | null }> {
  const plain = (tasks: Task[]) => ({ tasks, filterGroups: [], filterError: null });
  switch (view.kind) {
    case "filter":
      try {
        const groups = await invoke("filter_view", { query: view.query });
        return { tasks: groups.flatMap((g) => g.tasks), filterGroups: groups, filterError: null };
      } catch (e) {
        return { tasks: [], filterGroups: [], filterError: String(e) };
      }
    case "upcoming":
      // Overdue (from the Today query) on top, then the window.
      return plain(upcomingTasks(todayTasks, await invoke("upcoming_view", { days: UPCOMING_DAYS }), today));
    case "today":
      return plain(todayTasks);
    case "inbox":
      return plain(await invoke("project_view", { project_id: inbox.id }));
    case "project":
      return plain(await invoke("project_view", { project_id: view.id }));
    case "label":
      return plain(await invoke("label_view", { name: view.name }));
    case "search":
      // The Search view loads its own results.
      return plain([]);
  }
}

function completedFor(view: View, inbox: Project): Promise<Task[]> {
  switch (view.kind) {
    case "today":
      return invoke("completed_today", {});
    case "inbox":
      return invoke("completed_in_project", { project_id: inbox.id });
    case "project":
      return invoke("completed_in_project", { project_id: view.id });
    case "label":
      return invoke("completed_with_label", { name: view.name });
    default:
      return Promise.resolve([]);
  }
}

export async function loadView(view: View, showCompleted: boolean, today: string): Promise<Loaded> {
  const projects = await invoke("list_projects", {});
  const inbox = projects.find((p) => p.is_inbox);
  if (!inbox) throw "no inbox project";
  const [sections, labels, reminders, filters, todayTasks, all] = await Promise.all([
    Promise.all(projects.map((p) => invoke("list_sections", { project_id: p.id }))).then((s) => s.flat()),
    invoke("list_labels", {}),
    invoke("list_reminders", {}),
    invoke("list_filters", {}),
    invoke("today_view", {}),
    invoke("all_tasks", {}),
  ]);
  const { tasks, filterGroups, filterError } = await viewTasks(view, inbox, todayTasks, today);
  const completed = showCompleted ? await completedFor(view, inbox) : [];
  return {
    view,
    projects,
    sections,
    labels,
    filters,
    inbox,
    todayCount: todayTasks.length,
    reminders,
    tasks,
    all,
    completed,
    filterGroups,
    filterError,
  };
}

export type LoadState = { loaded: Loaded | null; error: string | null };

/**
 * Loads `view` again whenever it, `showCompleted` or `reloadKey` changes.
 * Only the latest request lands: a slow load for an earlier view is dropped.
 * The previous data stays shown until the new one arrives.
 */
export function useLoaded(
  view: View,
  showCompleted: boolean,
  reloadKey: number,
  load: (view: View, showCompleted: boolean, today: string) => Promise<Loaded> = loadView,
): LoadState {
  const [state, setState] = useState<LoadState>({ loaded: null, error: null });
  const latest = useRef(0);
  useEffect(() => {
    // reloadKey is only a trigger: a bump reloads the same view.
    void reloadKey;
    const mine = ++latest.current;
    load(view, showCompleted, localToday()).then(
      (loaded) => {
        if (latest.current === mine) setState({ loaded, error: null });
      },
      (e: unknown) => {
        if (latest.current === mine) setState((s) => ({ loaded: s.loaded, error: String(e) }));
      },
    );
  }, [view, showCompleted, reloadKey, load]);
  return state;
}
